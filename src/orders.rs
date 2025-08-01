use std::{fmt::Display, result, str::FromStr};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sqlx::{Encode, prelude::FromRow};

use crate::db::Db;

#[derive(Debug, Serialize, Deserialize, FromRow, Default)]
pub struct Order {
    pub id: Option<i64>,
    pub amount: i64,
    pub status: OrderStatus,
}

impl Order {
    pub fn new(amount: i64) -> Self {
        Self {
            amount,
            ..Default::default()
        }
    }

    pub async fn save(&mut self, db: &Db) -> Result<()> {
        let status = &self.status.to_string();

        match self.id {
            None => {
                let result = sqlx::query!(
                    "INSERT INTO orders (status, amount) VALUES (?, ?);",
                    status,
                    self.amount
                )
                .execute(db)
                .await?;

                self.id = Some(result.last_insert_rowid());
            }
            Some(id) => {
                sqlx::query!(
                    "update orders set status = ?, amount = ? where id = ?;",
                    status,
                    self.amount,
                    id
                ).execute(db).await?;
            }
        }

        Ok(())
    }

    pub async fn get_by_id(db: &Db, id: i64) -> Result<Option<Self>> {
        Ok(
            sqlx::query_as!(Order, "select * from orders where id = ?", id)
                .fetch_optional(db)
                .await?,
        )
    }

    pub async fn get_all(db: &Db) -> Result<Vec<Self>> {
        Ok(sqlx::query_as!(Order, "select * from orders")
            .fetch_all(db)
            .await?)
    }
}

#[derive(Debug, Serialize, Deserialize, Encode, PartialEq, Eq)]
pub enum OrderStatus {
    Pending,
    InProgress,
    Complete,
    Canceled,
}

impl Default for OrderStatus {
    fn default() -> Self {
        OrderStatus::Pending
    }
}

impl ToString for OrderStatus {
    fn to_string(&self) -> String {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::InProgress => "in-progress",
            OrderStatus::Complete => "complete",
            OrderStatus::Canceled => "canceled",
        }
        .to_string()
    }
}

impl From<String> for OrderStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pending" => OrderStatus::Pending,
            "in-progress" => OrderStatus::InProgress,
            "complete" => OrderStatus::Complete,
            "canceled" => OrderStatus::Canceled,
            _ => OrderStatus::default(),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::db::test_db;

    use super::*;

    #[tokio::test]
    async fn test_save_and_get_order() {
        let db = test_db().await;

        let mut order = Order::new(500);

        order
            .save(&db)
            .await
            .expect("order should save without error");

        let fresh_order =
            Order::get_by_id(&db, order.id.expect("order should have id after saved"))
                .await
                .expect("query should run without error")
                .expect("order should have been found");

        assert_eq!(order.amount, fresh_order.amount);


        order.amount = 900;

        order
            .save(&db)
            .await
            .expect("order should save without error");

        let fresh_order =
            Order::get_by_id(&db, order.id.expect("order should have id after saved"))
                .await
                .expect("query should run without error")
                .expect("order should have been found");

        assert_eq!(900, fresh_order.amount);
    }

    #[tokio::test]
    async fn test_get_by_id_none_if_not_exist() {
        let db = test_db().await;

        let result = Order::get_by_id(&db, 999).await.expect("should not error");

        assert!(result.is_none());
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

        let results = Order::get_all(&db).await.expect("should not error");

        assert_eq!(results.len(), 5);
    }
}
