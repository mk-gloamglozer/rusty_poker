use actix_web::web::{Data, Path};
use actix_web::{web, App, HttpResponse, HttpServer};
use poker_board::command::adapter::{ArcMutexStore, CombinedEventStore, DefaultStore, NoRetry};
use poker_board::command::event::{
    BoardModifiedEvent, CombinedEvent, VoteTypeEvent, VoteValidation,
};
use poker_board::command::BoardCommand;
use std::fmt::Debug;
use util::query::Query;
use util::use_case::UseCase;

use actix::{Actor, ActorContext, Addr, Handler, Recipient, StreamHandler};
use actix_web_actors::ws;
use bin::{ArcWsServer, BoardId, Session, SessionId};

async fn board_ws(
    r: actix_web::HttpRequest,
    stream: actix_web::web::Payload,
    path: Path<String>,
    data: Data<Addr<ArcWsServer>>,
) -> actix_web::Result<actix_web::HttpResponse> {
    // generate a new session id
    let board_id = BoardId::new(path.into_inner());
    ws::start(
        Session::new(SessionId::new(), board_id, data.get_ref().clone()),
        &r,
        stream,
    )
    .log()
}

trait Log {
    fn log(self) -> Self;
}

impl<T> Log for T
where
    T: Debug,
{
    fn log(self) -> Self {
        log::info!("Log: {:?}", self);
        self
    }
}

#[actix_web::post("/board/{id}")]
async fn clear_votes(
    data: Data<UseCase<CombinedEvent>>,
    body: String,
    path: Path<String>,
) -> HttpResponse {
    let command = match serde_json::from_str::<BoardCommand>(&body) {
        Ok(body) => body,
        Err(err) => {
            log::error!("Error parsing body: {}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    let key = path.into_inner();
    let response = data.execute(&key, &command).await;
    response
        .log()
        .map(|events| {
            serde_json::to_string(&events)
                .map(|body| HttpResponse::Ok().body(body))
                .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
        })
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[actix_web::get("/board/{id}")]
async fn get_events(query: Data<Query<BoardModifiedEvent>>, path: Path<String>) -> HttpResponse {
    let key = path.into_inner();
    log::debug!("Getting board with key: {}", key);
    let response = query.query::<poker_board::query::Board>(&key).await;
    response
        .log()
        .map(|board| HttpResponse::Ok().json(board))
        .unwrap_or_else(|_| HttpResponse::NotFound().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let board_modified_store = ArcMutexStore::<BoardModifiedEvent>::new();
    let vote_type_store = DefaultStore::<VoteTypeEvent>::new(vec![VoteTypeEvent::VoteTypeAdded {
        vote_validation: VoteValidation::AnyNumber,
        vote_type_id: "1".to_string(),
    }]);

    let store = || -> CombinedEventStore {
        CombinedEventStore::new(
            board_modified_store.clone(),
            vote_type_store.clone(),
            board_modified_store.clone(),
        )
    };

    let transaction = util::transaction::Transaction::new(NoRetry::new(), store(), store());

    let use_case = UseCase::new(transaction);
    let query = Query::<BoardModifiedEvent>::new(store());

    let use_case_data = Data::new(use_case);
    let query_data = Data::new(query);

    let server = ArcWsServer::new(store()).start();

    HttpServer::new(move || {
        App::new()
            .route("/ws/board/{id}", web::get().to(board_ws))
            .app_data(Data::new(server.clone()))
            .app_data(query_data.clone())
            .app_data(use_case_data.clone())
            .service(clear_votes)
            .service(get_events)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
