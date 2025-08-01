use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use db::Db;
use error::{CustomError, Result};
use orders::{Order, OrderStatus};
use serde::{Deserialize, Serialize};

mod db;
mod error;
mod orders;

#[derive(Clone)]
struct AppState {
    db: Arc<Db>,
}

#[tokio::main]
async fn main() {
    let db = db::setup_db().await;

    let app = app(db);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn app(db: Db) -> Router {
    let state = AppState { db: Arc::new(db) };

    Router::new()
        .route("/orders", get(get_orders).post(create_order))
        .route(
            "/orders/{id}",
            get(get_order_by_id).patch(update_order_status).delete(delete_order),
        )
        .with_state(state)
}

async fn get_orders(State(state): State<AppState>) -> Result<Json<Vec<Order>>> {
    let db = &state.db;

    let orders = Order::get_all(db).await?;

    Ok(Json(orders))
}

async fn get_order_by_id(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Order>> {
    let db = &state.db;

    match Order::get_by_id(db, id).await? {
        Some(order) => Ok(Json(order)),
        None => Err(CustomError::RecordNotFound),
    }
}

async fn create_order(
    State(state): State<AppState>,
    Json(mut order): Json<Order>,
) -> Result<Json<Order>> {
    let db = &state.db;

    order.save(db).await?;

    Ok(Json(order))
}

#[derive(Debug, Deserialize, Serialize)]
struct UpdateOrderStatusRequest {
    status: OrderStatus,
}

async fn update_order_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateOrderStatusRequest>,
) -> Result<()> {
    let db = &state.db;

    match Order::get_by_id(db, id).await? {
        Some(mut order) => {
            order.status = body.status;
            order.save(db).await?;

            Ok(())
        }
        None => Err(CustomError::RecordNotFound),
    }
}

async fn delete_order(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<()> {
    let db = &state.db;

    match Order::delete_by_id(db, id).await? {
        true => Ok(()),
        false => Err(CustomError::RecordNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use db::test_db;
    use http_body_util::BodyExt;
    use sqlx::sqlite::SqlitePoolOptions;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_create_order() {
        let app = app(test_db().await);
        let body = serde_json::to_string(&Order::new(500)).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .uri("/orders")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let order = serde_json::from_slice::<Order>(&body).expect("should serialise into an order");

        assert!(order.id.is_some());
    }

    #[tokio::test]
    async fn test_create_order_bad_input() {
        let app = app(test_db().await);
        let body = serde_json::json!({
            "amount": "invalid amount",
            "status": "invalid status",
        })
        .to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .uri("/orders")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();

        // should say amount can't be deserialized
        assert!(body.contains("amount"));
    }

    #[tokio::test]
    async fn test_update_order_status() {
        let db = test_db().await;

        let mut order = Order::new(500);

        order
            .save(&db)
            .await
            .expect("order should save without error");

        let app = app(db.clone());
        let body = serde_json::to_string(&UpdateOrderStatusRequest {
            status: OrderStatus::Complete,
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .header("Content-Type", "application/json")
                    .uri(format!(
                        "/orders/{}",
                        order.id.expect("should have id after save()")
                    ))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let order = Order::get_by_id(&db, order.id.unwrap()).await.unwrap();

        assert_eq!(order.unwrap().status, OrderStatus::Complete);
    }

    #[tokio::test]
    async fn test_update_order_status_not_found() {
        let db = test_db().await;
        let app = app(db);
        let body = serde_json::to_string(&UpdateOrderStatusRequest {
            status: OrderStatus::Complete,
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .header("Content-Type", "application/json")
                    .uri("/orders/999")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_order_status_bad_input() {
        let db = test_db().await;

        let mut order = Order::new(500);

        order
            .save(&db)
            .await
            .expect("order should save without error");

        let app = app(db);
        let body = serde_json::json!({
            "status": "invalid-status"
        }).to_string();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .header("Content-Type", "application/json")
                    .uri(format!(
                        "/orders/{}",
                        order.id.expect("should have id after save()")
                    ))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();

        // should say unknown variant for the enum
        assert!(body.contains("unknown variant"));
    }


    #[tokio::test]
    async fn test_get_order_by_id() {
        let db = test_db().await;

        let mut order = Order::new(500);

        order
            .save(&db)
            .await
            .expect("order should save without error");

        let app = app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!(
                        "/orders/{}",
                        order.id.expect("should have id after save()")
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let response_order =
            serde_json::from_slice::<Order>(&body).expect("should serialise into an order");

        assert_eq!(response_order.id, order.id);
    }

    #[tokio::test]
    async fn test_get_order_by_id_not_found() {
        let db = test_db().await;
        let app = app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/orders/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_all_orders() {
        let db = test_db().await;

        for _ in 0..5 {
            let mut order = Order::new(500);

            order
                .save(&db)
                .await
                .expect("order should save without error");
        }

        let app = app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/orders")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let orders =
            serde_json::from_slice::<Vec<Order>>(&body).expect("should serialise into an order");

        assert_eq!(orders.len(), 5);
    }

    #[tokio::test]
    async fn test_delete_order() {
        let db = test_db().await;

        let mut order = Order::new(500);
        order
            .save(&db)
            .await
            .expect("order should save without error");

        let app = app(db.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!(
                        "/orders/{}",
                        order.id.expect("should have id after save()")
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let result = Order::get_by_id(&db, order.id.unwrap()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_order_not_found() {
        let db = test_db().await;
        let app = app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orders/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }



    #[tokio::test]
    async fn test_server_error() {
        // create a database but don't run migrations to get queries to fail and cause a 500
        let db = SqlitePoolOptions::new().connect(":memory:").await.unwrap();

        let app = app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/orders")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();

        assert!(body.contains("Something went wrong!"));
    }



}
