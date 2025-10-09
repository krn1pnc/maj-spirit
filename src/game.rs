use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::txmanager::TxManager;
use crate::ws::{ClientMessage, GameInfo, ServerMessage};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cards {
    #[serde(with = "serde_bytes")]
    m: [u8; 34],
}

impl Deref for Cards {
    type Target = [u8; 34];

    fn deref(&self) -> &Self::Target {
        return &self.m;
    }
}

impl DerefMut for Cards {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.m;
    }
}

impl Default for Cards {
    fn default() -> Self {
        Self { m: [0; 34] }
    }
}

impl Display for Cards {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut res = String::with_capacity(14);
        for i in 0..34 {
            for _ in 0..self[i] {
                res.push(Cards::card_name(i as u8));
            }
        }
        return write!(f, "{}", res);
    }
}

impl Cards {
    pub fn new(cards: [u8; 34]) -> Cards {
        return Cards { m: cards };
    }

    fn copy_insert(&self, card: u8) -> Cards {
        let mut res = self.clone();
        res[card as usize] += 1;
        return res;
    }

    pub fn insert(&mut self, card: u8) {
        self[card as usize] += 1;
    }

    pub fn delete(&mut self, card: u8) {
        self[card as usize] -= 1;
    }

    pub fn card_name(card: u8) -> char {
        return "壹贰叁肆伍陆柒捌玖一二三四五六七八九123456789东南西北白发中"
            .chars()
            .nth(card as usize)
            .unwrap();
    }

    pub fn card_id(name: char) -> Option<u8> {
        return "壹贰叁肆伍陆柒捌玖一二三四五六七八九123456789东南西北白发中"
            .chars()
            .position(|x| x == name)
            .map(|x| x as u8);
    }
}

struct Stack {
    stack: [u8; 136],
    next: usize,
}

impl Stack {
    fn random() -> Stack {
        let mut stack = [0 as u8; 136];
        for i in 0..34 {
            for j in 0..4 {
                stack[i * 4 + j] = i as u8;
            }
        }
        stack.shuffle(&mut rand::rng());
        return Stack {
            stack: stack,
            next: 0,
        };
    }
    fn next(&mut self) -> u8 {
        self.next += 1;
        return self.stack[self.next - 1];
    }
}

pub struct Round {
    stack: Stack,
    current_player: usize,
    players_cards: [Cards; 4],
}

impl Round {
    pub fn new(host: usize) -> Round {
        let mut stack = Stack::random();
        let mut players_cards = [Cards::default(); 4];
        for _ in 0..13 {
            for i in 0..4 {
                players_cards[i].insert(stack.next());
            }
        }
        players_cards[host].insert(stack.next());
        return Round {
            stack,
            current_player: host,
            players_cards,
        };
    }
}

pub struct RoundRecord {
    pub stack: [u8; 136],
    pub winner_seat: Option<usize>,
    pub loser_seat: Option<usize>,
    pub discard: Vec<u8>,
}

pub struct Game {
    pub round: Round,
    pub round_id: usize,
    pub players: [u64; 4],
    pub players_score: [i64; 4],
    pub conn: Arc<RwLock<TxManager<u64, ServerMessage>>>,

    pub round_records: Vec<RoundRecord>,
}

impl Game {
    pub fn new(players: [u64; 4], conn: Arc<RwLock<TxManager<u64, ServerMessage>>>) -> Game {
        let game = Game {
            round: Round::new(0),
            round_id: 0,
            players,
            players_score: [0; 4],
            conn,
            round_records: Vec::with_capacity(4),
        };
        return game;
    }

    async fn send(&self, player: usize, msg: ServerMessage) {
        match self.conn.read().await.send(&self.players[player], msg) {
            Err(e) => tracing::error!("{:?}", e),
            Ok(_) => (),
        }
    }

    pub async fn broadcast(&self, msg: ServerMessage) {
        for j in 0..4 {
            self.send(j, msg.clone()).await;
        }
    }

    pub async fn game_start(&mut self) {
        let game_info = GameInfo {
            round_id: self.round_id,
            players: self.players,
            players_score: self.players_score,
        };
        self.broadcast(ServerMessage::GameInfoSync(game_info)).await;
        self.round_start().await;
    }

    pub async fn round_start(&mut self) {
        for i in 0..4 {
            self.send(i, ServerMessage::RoundStart(self.round_id)).await;
            self.send(i, ServerMessage::CardSync(self.round.players_cards[i]))
                .await;
        }
        self.round_records.push(RoundRecord {
            stack: self.round.stack.stack,
            winner_seat: None,
            loser_seat: None,
            discard: Vec::new(),
        });
    }

    async fn next_round(&mut self) -> bool {
        self.round_id += 1;

        // check game end
        if self.round_id == 4 {
            return true;
        }

        self.round = Round::new(self.round_id);
        self.round_start().await;
        return false;
    }

    async fn tie(&mut self) -> bool {
        self.broadcast(ServerMessage::Tie).await;
        return self.next_round().await;
    }

    async fn win_one(&mut self, win_player: usize, lose_player: usize) -> bool {
        // process score change
        self.players_score[win_player] += 1;
        self.players_score[lose_player] -= 1;

        // broadcast win message
        self.broadcast(ServerMessage::WinOne((
            self.players[win_player],
            self.players[lose_player],
        )))
        .await;

        // record win
        if let Some(record) = self.round_records.last_mut() {
            record.winner_seat = Some(win_player);
            record.loser_seat = Some(lose_player);
        } else {
            tracing::error!("this should not happen");
        }

        // prepare next round / end game
        return self.next_round().await;
    }

    async fn win_all(&mut self, win_player: usize) -> bool {
        // process score change
        self.players_score[win_player] += 3;
        for i in 0..4 {
            if i != win_player {
                self.players_score[i] -= 1;
            }
        }

        // broadcast win message
        self.broadcast(ServerMessage::WinAll(self.players[win_player]))
            .await;

        // record win
        if let Some(record) = self.round_records.last_mut() {
            record.winner_seat = Some(win_player);
        } else {
            tracing::error!("this should not happen");
        }

        // prepare next round / end game
        return self.next_round().await;
    }

    pub async fn handle_message(&mut self, msg: ClientMessage, uid: u64) -> bool {
        tracing::debug!("handle msg {:?} from {}", msg, uid);
        let mut player = None;
        for i in 0..4 {
            if self.players[i] == uid {
                player = Some(i);
            }
        }
        let player = player.unwrap();

        match msg {
            ClientMessage::RequestGameSync => {
                let game_info = GameInfo {
                    round_id: self.round_id,
                    players: self.players,
                    players_score: self.players_score,
                };
                self.send(player, ServerMessage::GameInfoSync(game_info))
                    .await;
                return false;
            }
            ClientMessage::RequestCardSync => {
                let cards = self.round.players_cards[player];
                self.send(player, ServerMessage::CardSync(cards)).await;
                return false;
            }
            ClientMessage::Discard(card) => {
                if player != self.round.current_player {
                    self.send(player, ServerMessage::NotCurrentPlayer).await;
                    return false;
                }

                // check if the card can be discard
                if self.round.players_cards[player][card as usize] == 0 {
                    self.send(player, ServerMessage::NotHaveCard).await;
                    return false;
                }

                // broadcast discard
                self.broadcast(ServerMessage::Discard((self.players[player], card)))
                    .await;

                // discard
                self.round.players_cards[player].delete(card);

                // record discard
                if let Some(record) = self.round_records.last_mut() {
                    record.discard.push(card);
                } else {
                    tracing::error!("this should not happen");
                }

                // check win one
                for i in 1..4 {
                    let check_player = (player + i) % 4;
                    if check_win::check(&self.round.players_cards[check_player].copy_insert(card)) {
                        return self.win_one(check_player, player).await;
                    }
                }

                // check tie
                if self.round.stack.next == 136 {
                    return self.tie().await;
                }

                // get next card
                let next_card = self.round.stack.next();
                let next_player = (player + 1) % 4;
                self.round.players_cards[next_player].insert(next_card);
                self.send(next_player, ServerMessage::GetCard(next_card))
                    .await;

                // check win all
                if check_win::check(&self.round.players_cards[next_player]) {
                    return self.win_all(next_player).await;
                }

                // maintain current_player
                self.round.current_player = next_player;
                return false;
            }
        }
    }
}

pub mod check_win {
    use crate::game::Cards;

    type State = [[i32; 3]; 3];

    fn transition(u: &State, x: i32) -> State {
        let mut v = [[-1; 3]; 3];
        for i in 0..3 {
            for j in (0..3).take_while(|&j| u[i][j] != -1) {
                for k in (0..3).take_while(|&k| i + j + k <= x as usize) {
                    let nv = u[i][j] + i as i32 + (x as usize >= 3 + i + j + k) as i32;
                    v[j][k] = v[j][k].max(nv);
                }
            }
        }
        return v;
    }

    fn next_same_suit(f0: &mut State, f1: &mut State, count: &mut i32, x: i32) {
        *f0 = transition(f0, x);
        *f1 = transition(f1, x);
        if x >= 2 {
            *count += 1;
            let nf1 = transition(f0, x - 2);
            for i in 0..3 {
                for j in 0..3 {
                    f1[i][j] = f1[i][j].max(nf1[i][j]);
                }
            }
        }
    }

    fn switch_suit(f0: &mut State, f1: &mut State) {
        for i in 0..3 {
            for j in 0..3 {
                if i == 0 && j == 0 {
                    continue;
                }
                f0[i][j] = -1;
                f1[i][j] = -1;
            }
        }
    }

    pub fn check(cards: &Cards) -> bool {
        let mut f0 = [[-1; 3]; 3];
        let mut f1 = [[-1; 3]; 3];
        let mut count = 0;

        f0[0][0] = 0;
        for idx in 0..9 {
            next_same_suit(&mut f0, &mut f1, &mut count, cards[idx] as i32);
        }
        switch_suit(&mut f0, &mut f1);
        for idx in 9..18 {
            next_same_suit(&mut f0, &mut f1, &mut count, cards[idx] as i32);
        }
        switch_suit(&mut f0, &mut f1);
        for idx in 18..27 {
            next_same_suit(&mut f0, &mut f1, &mut count, cards[idx] as i32);
        }
        switch_suit(&mut f0, &mut f1);
        for idx in 27..34 {
            next_same_suit(&mut f0, &mut f1, &mut count, cards[idx] as i32);
            switch_suit(&mut f0, &mut f1);
        }

        let mut mx = 0;
        for i in 0..3 {
            for j in 0..3 {
                mx = mx.max(f1[i][j]);
            }
        }
        return count >= 7 || mx >= 4;
    }
}
