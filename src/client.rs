use std::{io::Write, sync::Arc, u8};

use futures_util::{SinkExt, StreamExt};
use maj_spirit::{
    game::Cards,
    ws::{ClientMessage, ServerMessage},
};
use nyquest::{BlockingClient, ClientBuilder, blocking::Request, body_form};
use tokio::sync::{RwLock, mpsc};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("nyquest error: {0}")]
    Nyquest(#[from] nyquest::Error),

    #[error("server error: {0}")]
    Server(String),

    #[error("password incorrect")]
    PasswordIncorrect,
}

fn read_line() -> String {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    return input;
}

fn prompt(s: &str) {
    print!("{}", s);
    std::io::stdout().flush().unwrap();
}

fn login(
    client: &BlockingClient,
    base_url: &str,
    username: &str,
    password: &str,
) -> Result<String, ClientError> {
    let login_resp = client.request(Request::post(format!("{}/login", base_url)).with_body(
        body_form! {
            "username" => username.to_string(),
            "password" => password.to_string(),
        },
    ))?;

    if login_resp.status() != 200 {
        let msg = login_resp.text()?;
        match msg.as_ref() {
            "user not exist" => {
                println!("正在注册新账号\n");

                let register_resp = client.request(
                    Request::post(format!("{}/register", base_url)).with_body(body_form! {
                        "username" => username.to_string(),
                        "password" => password.to_string(),
                    }),
                )?;

                if register_resp.status() != 200 {
                    return Err(ClientError::Server(register_resp.text()?));
                }

                let login_resp = client.request(
                    Request::post(format!("{}/login", base_url)).with_body(body_form! {
                        "username" => username.to_string(),
                        "password" => password.to_string(),
                    }),
                )?;

                return Ok(login_resp.text()?);
            }
            "passord incorrect" => return Err(ClientError::PasswordIncorrect),
            msg => return Err(ClientError::Server(msg.to_string())),
        }
    } else {
        return Ok(login_resp.text()?);
    }
}

#[tokio::main]
async fn main() {
    nyquest_preset::register();

    let client = ClientBuilder::default().build_blocking().unwrap();

    prompt("请输入服务器地址，直接回车默认为 127.0.0.1:3000：");
    let mut addr = read_line().trim().to_owned();

    if addr == "" {
        addr = String::from("127.0.0.1:3000");
    }

    let base_url = format!("http://{}", addr);
    let ws_url = format!("ws://{}/ws", addr);

    prompt("请输入用户名：");
    let username = read_line().trim().to_owned();
    prompt("请输入密码：");
    let password = rpassword::read_password().unwrap();

    let token = login(&client, &base_url, &username, &password).unwrap();
    let auth_header = format!("Bearer {}", token);
    let mut ws_request = ws_url.into_client_request().unwrap();
    ws_request
        .headers_mut()
        .insert("Authorization", auth_header.clone().parse().unwrap());

    let (ws_stream, ws_resp) = connect_async(ws_request.clone()).await.unwrap();

    if ws_resp.status().as_u16() > 299 {
        println!("WebSocket 连接失败");
        return;
    }

    println!("WebSocket 连接成功");

    let (mut tx, mut rx) = ws_stream.split();

    let (send_tx, mut send_rx) = mpsc::unbounded_channel::<ClientMessage>();

    tokio::spawn(async move {
        while let Some(msg) = send_rx.recv().await {
            let msg = serde_json::to_string(&msg).unwrap();
            tx.send(Message::Text(msg.into())).await.unwrap();
        }
    });

    let current_cards = Arc::new(RwLock::new(Cards::default()));
    let is_auto = Arc::new(RwLock::new(false));

    let send_tx_ = send_tx.clone();
    let currnet_cards_ = current_cards.clone();
    let is_auto_ = is_auto.clone();
    tokio::spawn(async move {
        let send_tx = send_tx_;
        let current_cards = currnet_cards_;
        let is_auto = is_auto_;
        while let Some(msg) = rx.next().await {
            if let Ok(Message::Text(msg)) = msg {
                println!("recv {}", msg);
                let msg: ServerMessage = serde_json::from_str(&msg).unwrap();
                match msg {
                    ServerMessage::GameNotStart => {
                        println!("游戏尚未启动");
                    }
                    ServerMessage::UserNotInRoom => {
                        println!("你不在房间内");
                    }
                    ServerMessage::NotCurrentPlayer => {
                        println!("你当前不能出牌");
                    }
                    ServerMessage::GetCard(card) => {
                        println!("你获得了：{}", Cards::card_name(card));
                        current_cards.write().await.insert(card);
                        println!("你的牌是：{}", current_cards.read().await);

                        if *is_auto.read().await {
                            let cards = current_cards.read().await;
                            let c = cards.into_iter().position(|x| x > 0).unwrap();
                            send_tx.send(ClientMessage::Discard(c as u8)).unwrap();
                        }
                    }
                    ServerMessage::NotHaveCard => {
                        println!("你没有足够的牌");
                    }
                    ServerMessage::Discard((uid, card)) => {
                        println!("玩家 {} 打出了：{}", uid, Cards::card_name(card));
                        if uid == username {
                            current_cards.write().await.delete(card);
                        }
                    }
                    ServerMessage::RoundStart((uid, cards)) => {
                        println!("本轮开始，玩家 {} 是庄家", uid);
                        println!("你的牌是：{}", cards);
                        *current_cards.write().await = cards;

                        if uid == username && *is_auto.read().await {
                            let cards = current_cards.read().await;
                            let c = cards.into_iter().position(|x| x > 0).unwrap();
                            send_tx.send(ClientMessage::Discard(c as u8)).unwrap();
                        }
                    }
                    ServerMessage::WinAll(uid) => {
                        println!("玩家 {} 自摸", uid);
                    }
                    ServerMessage::WinOne((win_uid, lose_uid)) => {
                        println!("玩家 {} 荣和，倒霉蛋是 {}", win_uid, lose_uid);
                    }
                    ServerMessage::Tie => {
                        println!("流局");
                    }
                    ServerMessage::GameEnd(game_id) => {
                        println!("游戏结束，对局 id 是 {}", game_id);
                    }
                }
            }
        }
    });

    loop {
        let cmd = read_line().trim().to_owned();
        let cmd: Vec<&str> = cmd.split(' ').collect();
        match cmd[0] {
            "room" => {
                if cmd.len() < 2 {
                    println!("不合法的命令");
                } else {
                    match cmd[1] {
                        "join" | "leave" | "start" => {
                            if cmd.len() != 3 {
                                println!("不合法的命令");
                            } else {
                                let req = Request::post(format!(
                                    "{}/room/{}/{}",
                                    base_url, cmd[2], cmd[1]
                                ))
                                .with_header("Authorization", auth_header.clone());
                                let resp = client.request(req).unwrap();
                                let resp_debug = format!("{:?}", resp);
                                let resp_text = resp.text().unwrap();
                                if resp_text.len() != 0 {
                                    println!("{}", resp_text);
                                } else {
                                    println!("{}", resp_debug);
                                }
                            }
                        }
                        _ => {
                            println!("不合法的命令");
                        }
                    }
                }
            }
            "d" | "discard" => {
                if cmd.len() != 2 {
                    println!("不合法的命令");
                } else {
                    let card = Cards::card_id(cmd[1].chars().nth(0).unwrap());
                    send_tx.send(ClientMessage::Discard(card)).unwrap();
                }
            }
            "auto" => {
                if cmd.len() != 1 {
                    println!("不合法的命令");
                } else {
                    let mut is_auto = is_auto.write().await;
                    *is_auto = !*is_auto;
                    let status = if *is_auto { "开启" } else { "关闭" };
                    println!("代理已{}", status);
                }
            }
            _ => {
                println!("不合法的命令");
            }
        }
    }
}
