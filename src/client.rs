use std::io::Write;

use futures_util::{SinkExt, StreamExt};
use maj_spirit::{
    game::Cards,
    ws::{ClientMessage, ServerMessage},
};
use nyquest::{ClientBuilder, blocking::Request, body_form};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

fn read_line() -> String {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    return input;
}

fn prompt(s: &str) {
    print!("{}", s);
    std::io::stdout().flush().unwrap();
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

    let login_resp = client
        .request(
            Request::post(format!("{}/login", base_url)).with_body(body_form! {
                "username" => username.clone(),
                "password" => password.clone(),
            }),
        )
        .unwrap();

    let token;
    if login_resp.status() != 200 {
        let msg = login_resp.text().unwrap();
        match msg.as_ref() {
            "user not exist" => {
                println!("正在注册新账号\n");

                let register_resp = client
                    .request(Request::post(format!("{}/register", base_url)).with_body(
                        body_form! {
                            "username" => username.clone(),
                            "password" => password.clone(),
                        },
                    ))
                    .unwrap();

                if register_resp.status() != 200 {
                    println!("发生错误：{}", register_resp.text().unwrap());
                    return;
                }

                let login_resp = client
                    .request(
                        Request::post(format!("{}/login", base_url)).with_body(body_form! {
                            "username" => username.clone(),
                            "password" => password.clone(),
                        }),
                    )
                    .unwrap();

                token = Some(login_resp.text().unwrap());
            }
            "passord incorrect" => {
                println!("密码不正确");
                return;
            }
            _ => {
                println!("发生错误：{}", msg);
                return;
            }
        }
    } else {
        token = Some(login_resp.text().unwrap());
    }

    let token = token.unwrap();
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

    tokio::spawn(async move {
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
                    }
                    ServerMessage::NotHaveCard => {
                        println!("你没有足够的牌");
                    }
                    ServerMessage::Discard((uid, card)) => {
                        println!("玩家 {} 打出了：{}", uid, Cards::card_name(card));
                    }
                    ServerMessage::RoundStart((uid, cards)) => {
                        println!("本轮开始，玩家 {} 是庄家", uid);
                        println!("你的牌是：{}", cards);
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
                                let resp = client.request(req);
                                println!("{:?}", resp);
                                println!("{}", resp.unwrap().text().unwrap());
                            }
                        }
                        _ => {
                            println!("不合法的命令");
                        }
                    }
                }
            }
            "discard" => {
                if cmd.len() != 2 {
                    println!("不合法的命令");
                } else {
                    let card = Cards::card_id(cmd[1].chars().nth(0).unwrap());
                    let msg = ClientMessage::Discard(card);
                    let msg = serde_json::to_string(&msg).unwrap();
                    tx.send(Message::Text(msg.into())).await.unwrap();
                }
            }
            _ => {
                println!("不合法的命令");
            }
        }
    }
}
