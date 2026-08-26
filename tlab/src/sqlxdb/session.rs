use futures_core::{future::BoxFuture, stream::BoxStream};

use sqlx::{Acquire, SqlStr};

use crate::sqlxdb::map_error;

#[derive(Debug)]
pub enum Session<'c, DB: sqlx::Database>
where
    DB: sqlx::Database,
{
    Pool(sqlx::Pool<DB>),
    Tx(sqlx::Transaction<'c, DB>),
    Conn(sqlx::pool::PoolConnection<DB>),
}

impl<'c, DB: sqlx::Database> Session<'c, DB> {
    pub async fn begin(&mut self) -> crate::Result<Session<'_, DB>> {
        let tx = match self {
            Session::Pool(pool) => pool.begin().await,
            Session::Tx(tx) => tx.begin().await,
            Session::Conn(conn) => conn.begin().await,
        }
        .map_err(map_error)?;
        Ok(Session::Tx(tx))
    }

    pub async fn commit(self) -> crate::Result<()> {
        match self {
            Session::Pool(_) => Ok(()),
            Session::Tx(tx) => {
                tx.commit().await.map_err(map_error)?;
                Ok(())
            }
            Session::Conn(_) => Ok(()),
        }
    }

    pub async fn rollback(self) -> crate::Result<()> {
        match self {
            Session::Pool(_) => Ok(()),
            Session::Tx(tx) => {
                tx.rollback().await.map_err(map_error)?;
                Ok(())
            }
            Session::Conn(_) => Ok(()),
        }
    }

    pub fn backend(&mut self) -> &mut Self {
        self
    }
}

impl<'h, 'c, DB: sqlx::Database> sqlx::Executor<'h> for &'h mut Session<'c, DB>
where
    for<'e> &'e mut DB::Connection: sqlx::Executor<'e, Database = DB>,
{
    type Database = DB;

    fn fetch_many<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxStream<
        'e,
        Result<
            sqlx::Either<
                <Self::Database as sqlx::Database>::QueryResult,
                <Self::Database as sqlx::Database>::Row,
            >,
            sqlx::Error,
        >,
    >
    where
        'c: 'e,
        'h: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            Session::Pool(pool) => pool.fetch_many(query),
            Session::Tx(tx) => tx.fetch_many(query),
            Session::Conn(conn) => conn.fetch_many(query),
        }
    }

    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<<Self::Database as sqlx::Database>::Row>, sqlx::Error>>
    where
        'c: 'e,
        'h: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self {
            Session::Pool(pool) => pool.fetch_optional(query),
            Session::Tx(tx) => tx.fetch_optional(query),
            Session::Conn(conn) => conn.fetch_optional(query),
        }
    }

    fn prepare_with<'e>(
        self,
        sql: SqlStr,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Self::Database as sqlx::Database>::Statement, sqlx::Error>>
    where
        'c: 'e,
        'h: 'e,
    {
        match self {
            Session::Pool(pool) => pool.prepare_with(sql, parameters),
            Session::Tx(tx) => tx.prepare_with(sql, parameters),
            Session::Conn(conn) => conn.prepare_with(sql, parameters),
        }
    }

    fn describe<'e>(
        self,
        sql: SqlStr,
    ) -> BoxFuture<'e, Result<sqlx::Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
        'h: 'e,
    {
        match self {
            Session::Pool(pool) => pool.describe(sql),
            Session::Tx(tx) => tx.describe(sql),
            Session::Conn(conn) => conn.describe(sql),
        }
    }
}
