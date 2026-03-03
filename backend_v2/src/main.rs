use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};

use bcrypt::{hash, verify, DEFAULT_COST};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone)]
struct AppState {
    db: PgPool,
    tx: Arc<broadcast::Sender<String>>,
}

#[derive(Deserialize)]
struct RegReq {
    un: String,
    pw: String,
    pk: String,
    av: Option<String>,
    encrypted_privkey: String,
    privkey_salt: String,
    privkey_iv: String,
}

#[derive(Serialize)]
struct UserRes {
    id: Uuid,
    un: String,
    pk: String,
    av: Option<String>
}

#[derive(Serialize)]
struct LoginRes {
    id: Uuid,
    un: String,
    pk: String,
    av: Option<String>,
    encrypted_privkey: Option<String>,
    privkey_salt: Option<String>,
    privkey_iv: Option<String>,
}


async fn do_register(st: web::Data<AppState>, req: web::Json<RegReq>) -> impl Responder {
    let hashed = hash(&req.pw, DEFAULT_COST).unwrap();
    let new_id = Uuid::new_v4();

    sqlx::query("INSERT INTO users (id, uname, pass, pubkey, avatar, encrypted_privkey, privkey_salt, privkey_iv) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
        .bind(&new_id).bind(&req.un).bind(&hashed).bind(&req.pk).bind(&req.av)
        .bind(&req.encrypted_privkey).bind(&req.privkey_salt).bind(&req.privkey_iv)
        .execute(&st.db).await.unwrap();

    HttpResponse::Ok().json(LoginRes {
        id: new_id,
        un: req.un.clone(),
        pk: req.pk.clone(),
        av: req.av.clone(),
        encrypted_privkey: Some(req.encrypted_privkey.clone()),
        privkey_salt: Some(req.privkey_salt.clone()),
        privkey_iv: Some(req.privkey_iv.clone()),
    })
}

#[derive(Deserialize)]
struct LogReq {
    un: String,
    pw: String
}

#[derive(FromRow)]
struct DbUser {
    id: Uuid,
    uname: String,
    pass: String,
    pubkey: String,
    avatar: Option<String>,
    encrypted_privkey: Option<String>,
    privkey_salt: Option<String>,
    privkey_iv: Option<String>,
}

async fn do_login(st: web::Data<AppState>, req: web::Json<LogReq>) -> impl Responder {
    let u = sqlx::query_as::<_, DbUser>("SELECT * FROM users WHERE uname = $1")
        .bind(&req.un)
        .fetch_optional(&st.db).await.unwrap();

    if u.is_none() {
        return HttpResponse::Forbidden().body("bad user");
    }
    let u = u.unwrap();
    if verify(&req.pw, &u.pass).unwrap() {
        HttpResponse::Ok().json(LoginRes {
            id: u.id,
            un: u.uname,
            pk: u.pubkey,
            av: u.avatar,
            encrypted_privkey: u.encrypted_privkey,
            privkey_salt: u.privkey_salt,
            privkey_iv: u.privkey_iv,
        })
    } else {
        HttpResponse::Forbidden().body("bad pass")
    }
}

#[derive(Deserialize)]
struct MakeServerReq {
    name: String,
    owner_id: Uuid
}

async fn make_server(st: web::Data<AppState>, req: web::Json<MakeServerReq>) -> impl Responder {
    let sid = Uuid::new_v4();
    sqlx::query("INSERT INTO servers (id, name, owner) VALUES ($1, $2, $3)")
        .bind(&sid).bind(&req.name).bind(&req.owner_id)
        .execute(&st.db).await.unwrap();

    sqlx::query("INSERT INTO members (user_id, server_id) VALUES ($1, $2)")
        .bind(&req.owner_id).bind(&sid)
        .execute(&st.db).await.unwrap();

    HttpResponse::Ok().body(sid.to_string())
}

#[derive(Deserialize)]
struct JoinServerReq {
    user_id: Uuid,
    server_id: Uuid
}

async fn join_server(st: web::Data<AppState>, req: web::Json<JoinServerReq>) -> impl Responder {
    sqlx::query("INSERT INTO members (user_id, server_id) VALUES ($1, $2)")
        .bind(&req.user_id).bind(&req.server_id)
        .execute(&st.db).await.unwrap();
    HttpResponse::Ok().body("ok")
}

#[derive(Deserialize)]
struct MakeChanReq {
    server_id: Uuid,
    name: String
}

async fn make_chan(st: web::Data<AppState>, req: web::Json<MakeChanReq>) -> impl Responder {
    let cid = Uuid::new_v4();
    sqlx::query("INSERT INTO channels (id, server_id, name) VALUES ($1, $2, $3)")
        .bind(&cid).bind(&req.server_id).bind(&req.name)
        .execute(&st.db).await.unwrap();
    HttpResponse::Ok().body(cid.to_string())
}

#[derive(Serialize, FromRow)]
struct Server {
    id: Uuid,
    name: String,
    owner: Uuid
}

async fn get_servers(st: web::Data<AppState>, uid: web::Path<Uuid>) -> impl Responder {
    let res = sqlx::query_as::<_, Server>("SELECT s.* FROM servers s JOIN members m ON s.id = m.server_id WHERE m.user_id = $1")
        .bind(uid.into_inner())
        .fetch_all(&st.db).await.unwrap();
    HttpResponse::Ok().json(res)
}

#[derive(Serialize, FromRow)]
struct Channel {
    id: Uuid,
    server_id: Uuid,
    name: String
}

async fn get_channels(st: web::Data<AppState>, sid: web::Path<Uuid>) -> impl Responder {
    let res = sqlx::query_as::<_, Channel>("SELECT * FROM channels WHERE server_id = $1")
        .bind(sid.into_inner())
        .fetch_all(&st.db).await.unwrap();
    HttpResponse::Ok().json(res)
}

#[derive(Serialize, FromRow)]
struct MsgRes {
    id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    encrypted_content: String,
    nonce: String,
    created_at: Option<DateTime<Utc>>
}

async fn get_msgs(st: web::Data<AppState>, cid: web::Path<Uuid>) -> impl Responder {
    let res = sqlx::query_as::<_, MsgRes>("SELECT * FROM v2_messages WHERE channel_id = $1 ORDER BY created_at ASC")
        .bind(cid.into_inner())
        .fetch_all(&st.db).await.unwrap();
    HttpResponse::Ok().json(res)
}

async fn get_users(st: web::Data<AppState>, sid: web::Path<Uuid>) -> impl Responder {
    let res = sqlx::query_as::<_, DbUser>("SELECT u.* FROM users u JOIN members m ON u.id = m.user_id WHERE m.server_id = $1")
        .bind(sid.into_inner())
        .fetch_all(&st.db).await.unwrap();

    let out: Vec<UserRes> = res.into_iter().map(|u| UserRes { id: u.id, un: u.uname, pk: u.pubkey, av: u.avatar }).collect();
    HttpResponse::Ok().json(out)
}

#[derive(Serialize, FromRow)]
struct PubKeyRes {
    id: Uuid,
    pubkey: String,
}

async fn get_pubkey(st: web::Data<AppState>, uid: web::Path<Uuid>) -> impl Responder {
    let res = sqlx::query_as::<_, PubKeyRes>("SELECT id, pubkey FROM users WHERE id = $1")
        .bind(uid.into_inner())
        .fetch_optional(&st.db).await.unwrap();
    match res {
        Some(r) => HttpResponse::Ok().json(r),
        None => HttpResponse::NotFound().body("no user")
    }
}

#[derive(Serialize, FromRow)]
struct PendingMember {
    user_id: Uuid,
    pubkey: String,
}

async fn get_pending_members(st: web::Data<AppState>, sid: web::Path<Uuid>) -> impl Responder {
    let res = sqlx::query_as::<_, PendingMember>(
        "SELECT m.user_id, u.pubkey FROM members m JOIN users u ON m.user_id = u.id WHERE m.server_id = $1 AND NOT EXISTS (SELECT 1 FROM server_keys sk WHERE sk.server_id = m.server_id AND sk.user_id = m.user_id)"
    )
    .bind(sid.into_inner())
    .fetch_all(&st.db).await.unwrap();
    HttpResponse::Ok().json(res)
}

#[derive(Deserialize)]
struct StoreKeyReq {
    server_id: Uuid,
    user_id: Uuid,
    encrypted_key: String,
}

async fn store_server_key(st: web::Data<AppState>, req: web::Json<StoreKeyReq>) -> impl Responder {
    let res = sqlx::query(
        "INSERT INTO server_keys (server_id, user_id, encrypted_key) VALUES ($1, $2, $3)
         ON CONFLICT (server_id, user_id) DO UPDATE SET encrypted_key = EXCLUDED.encrypted_key"
    )
    .bind(&req.server_id).bind(&req.user_id).bind(&req.encrypted_key)
    .execute(&st.db).await;

    match res {
        Ok(_) => HttpResponse::Ok().body("ok"),
        Err(_) => HttpResponse::InternalServerError().body("failed")
    }
}

#[derive(Serialize, FromRow)]
struct ServerKeyRes {
    encrypted_key: String,
}

async fn get_server_key(st: web::Data<AppState>, path: web::Path<(Uuid, Uuid)>) -> impl Responder {
    let (sid, uid) = path.into_inner();
    let res = sqlx::query_as::<_, ServerKeyRes>(
        "SELECT encrypted_key FROM server_keys WHERE server_id = $1 AND user_id = $2"
    )
    .bind(&sid).bind(&uid)
    .fetch_optional(&st.db).await.unwrap();

    match res {
        Some(k) => HttpResponse::Ok().json(k),
        None => HttpResponse::NotFound().body("no key")
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum WsMsg {
    Chat {
        channel_id: String,
        author_id: String,
        encrypted_content: String,
        nonce: String
    },
    KeyRequest {
        server_id: String,
        requester_id: String,
        requester_pubkey: String
    },
    KeyDelivered {
        target_user_id: String,
        server_id: String,
    }
}

async fn ws(req: HttpRequest, body: web::Payload, st: web::Data<AppState>) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    let tx = st.tx.clone();
    let mut rx = tx.subscribe();
    let db = st.db.clone();

    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                Some(Ok(m)) = msg_stream.recv() => {
                    if let actix_ws::Message::Text(t) = m {
                        println!("Received WS message: {}", t);
                        match serde_json::from_str::<WsMsg>(&t) {
                            Ok(parsed) => {
                                match parsed {
                                    WsMsg::Chat { channel_id, author_id, encrypted_content, nonce } => {
                                        match (Uuid::parse_str(&channel_id), Uuid::parse_str(&author_id)) {
                                            (Ok(cid), Ok(aid)) => {
                                                let mid = Uuid::new_v4();
                                                let res = sqlx::query("INSERT INTO v2_messages (id, channel_id, author_id, encrypted_content, nonce) VALUES ($1, $2, $3, $4, $5)")
                                                    .bind(&mid).bind(&cid).bind(&aid).bind(&encrypted_content).bind(&nonce)
                                                    .execute(&db).await;
                                                if let Err(e) = res {
                                                    println!("DB Error: {:?}", e);
                                                } else {
                                                    println!("DB inserted message successfully");
                                                }
                                                let _ = tx.send(t.to_string());
                                            }
                                            (err_cid, err_aid) => {
                                                println!("Failed to parse UUIDs: cid={:?}, aid={:?}", err_cid, err_aid);
                                            }
                                        }
                                    }
                                    _ => {
                                        let _ = tx.send(t.to_string());
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Failed to parse JSON: {:?}", e);
                            }
                        }
                    } else if let actix_ws::Message::Close(_) = m {
                        break;
                    }
                }
                Ok(bmsg) = rx.recv() => {
                    if session.text(bmsg).await.is_err() {
                        break;
                    }
                }
                else => break
            }
        }
    });

    Ok(response)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let u = std::env::var("DATABASE_URL").unwrap();
    let p = sqlx::PgPool::connect(&u).await.unwrap();
    sqlx::migrate!("./migrations").run(&p).await.unwrap();
    let (tx, _) = broadcast::channel(100);
    let st = AppState { db: p, tx: Arc::new(tx) };

    let port: u16 = std::env::var("PORT").unwrap().parse().unwrap();
    let host = std::env::var("HOST").unwrap();

    println!("backend on {}:{}", host, port);

    HttpServer::new(move || {
        let cors = Cors::default().allow_any_origin().allow_any_method().allow_any_header();
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(st.clone()))
            .route("/r", web::post().to(do_register))
            .route("/l", web::post().to(do_login))
            .route("/ms", web::post().to(make_server))
            .route("/js", web::post().to(join_server))
            .route("/mc", web::post().to(make_chan))
            .route("/s/{uid}", web::get().to(get_servers))
            .route("/c/{sid}", web::get().to(get_channels))
            .route("/m/{cid}", web::get().to(get_msgs))
            .route("/u/{sid}", web::get().to(get_users))
            .route("/pk/{uid}", web::get().to(get_pubkey))
            .route("/sk", web::post().to(store_server_key))
            .route("/sk/{sid}/{uid}", web::get().to(get_server_key))
            .route("/sk/pending/{sid}", web::get().to(get_pending_members))
            .route("/ws", web::get().to(ws))
            .service(Files::new("/", "../frontend_v2_leptos/dist").index_file("index.html"))
    })
    .bind((host, port))?
    .run()
    .await
}
