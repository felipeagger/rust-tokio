use std::env;
use std::{time::Duration, sync::Arc};
use actix_web::{HttpServer, App, web, HttpResponse, http::KeepAlive};
use deadpool_postgres::{Runtime, PoolConfig, Timeouts};
use deadpool_redis::{ConnectionInfo, RedisConnectionInfo, ConnectionAddr};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct DataDTO {
    ip: String,
    data: String
}

type APIResult = Result<HttpResponse, Box<dyn std::error::Error>>;
type AsyncVoidResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
//type QueueEvent = (String, web::Json<DataDTO>, Option<String>);
//type AppQueue = deadqueue::unlimited::Queue::<QueueEvent>;

#[actix_web::post("/set")]
async fn set_cache(
    redis_pool: web::Data<deadpool_redis::Pool>,
    payload: web::Json<DataDTO>,
    //queue: web::Data<Arc<AppQueue>>
) -> APIResult {
    
    let dto = DataDTO {
        ip: payload.ip.clone(),
        data: payload.data.clone()
    };

    let body = serde_json::to_string(&dto)?;
    let body_async = body.clone();
    {
        let mut redis_conn = redis_pool.get().await.expect("error getting redis conn");
        let _ = deadpool_redis::redis::cmd("SET")
            .arg(&[payload.ip.clone(), body_async]).query_async::<_, ()>(&mut redis_conn)
            .await;
    }

    //Ok(HttpResponse::Ok().body(body))
    //queue.push((id.clone(), payload, stack));
    
    Ok(
        HttpResponse::Ok()
            .append_header(("ResourceKey", payload.ip.clone()))
            .body(body)
            //.finish()
    )
}

#[actix_web::get("/get/{id}")]
async fn get_cache(id: web::Path<String>, redis_pool: web::Data<deadpool_redis::Pool>) -> APIResult {
    let id = id.to_string();
    
    let mut redis_conn = redis_pool.get().await?;
    match deadpool_redis::redis::cmd("GET").arg(&[id.clone()]).query_async::<_, String>(&mut redis_conn).await {
        Err(_) => (),
        Ok(bytes) => return Ok(HttpResponse::Ok().body(bytes))
    };
    
   
    Ok(HttpResponse::NotFound().finish())
}

#[tokio::main]
async fn main() -> AsyncVoidResult {

    let cache_host = env::var("CACHE_HOST").unwrap_or("127.0.0.1".to_string());

    let mut cfg = deadpool_redis::Config::default();
    cfg.connection = Some(ConnectionInfo {
        addr: ConnectionAddr::Tcp(cache_host.into(), 6379),
        redis: RedisConnectionInfo {
            db: 0,
            username: None,
            password: None
        }
    });
    cfg.pool = Some(PoolConfig {
        max_size: 9995,
        timeouts: Timeouts {
            wait: Some(Duration::from_secs(60)),
            create: Some(Duration::from_secs(60)),
            recycle: Some(Duration::from_secs(60))
        }
    });
    
    println!("creating redis pool...");
    let redis_pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    println!("redis pool succesfully created");

    //let queue = Arc::new(AppQueue::new());
    //let queue_async = queue.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(redis_pool.clone()))
            //.app_data(web::Data::new(queue.clone()))
            .service(set_cache)
            .service(get_cache)
    })
    .keep_alive(KeepAlive::Timeout(Duration::from_secs(200)))
    .bind("0.0.0.0:9090")?
    .run()
    .await?;

    Ok(())
}