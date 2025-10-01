# Maj Spirit

## Dev log

### 20250930

选题．感觉 MiniNginx 有点太魔怔了，什么叫配置文件解析要自己写？工程量太大，遂放弃．OIDC 没学过啊，而且锁 Go，没法写．看 MajSpirit，诶这不是我们 ZJOI 2019 吗，是我喜欢的题目，直接选择．

选型．语言神，启动！简单搜索后发现有个叫 tungstenite 的库可以提供 WebSocket 支持，并且可以和 tokio 一起用．初步想法是用这个处理 WebSocket，用 axum 处理 http．反正没有性能要求，数据库直接用熟悉的 SQLite 了．

### 20251001

早起回老家．路上细细搜索了一下，发现 axum 本身可以处理 WebSocket，底层用的就是 tungstenite．