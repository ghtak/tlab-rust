use futures_core::{future::BoxFuture, stream::BoxStream};

use sqlx::{Acquire, SqlStr};

use crate::db::map_error;

#[allow(dead_code)]
#[derive(Debug)]
pub struct BackendImpl<'h, 'c, DB>
where
    DB: sqlx::Database,
    for<'e> &'e mut DB::Connection: sqlx::Executor<'e, Database = DB>,
{
    pub handle: &'h mut SqlxSession<'c, DB>,
}

#[derive(Debug)]
pub enum SqlxSession<'c, DB>
where
    DB: sqlx::Database,
    for<'e> &'e mut DB::Connection: sqlx::Executor<'e, Database = DB>,
{
    Pool(sqlx::Pool<DB>),
    Tx(sqlx::Transaction<'c, DB>),
    Conn(sqlx::pool::PoolConnection<DB>),
}

impl<'c, DB> SqlxSession<'c, DB>
where
    DB: sqlx::Database,
    for<'e> &'e mut DB::Connection: sqlx::Executor<'e, Database = DB>,
{
    pub async fn begin(&mut self) -> crate::Result<SqlxSession<'_, DB>> {
        let tx = match self {
            SqlxSession::Pool(pool) => pool.begin().await,
            SqlxSession::Tx(tx) => tx.begin().await,
            SqlxSession::Conn(conn) => conn.begin().await,
        }
        .map_err(map_error)?;
        Ok(SqlxSession::Tx(tx))
    }

    pub async fn commit(self) -> crate::Result<()> {
        match self {
            SqlxSession::Pool(_) => Ok(()),
            SqlxSession::Tx(tx) => tx.commit().await.map_err(map_error),
            SqlxSession::Conn(_) => Ok(()),
        }
    }

    pub async fn rollback(self) -> crate::Result<()> {
        match self {
            SqlxSession::Pool(_) => Ok(()),
            SqlxSession::Tx(tx) => tx.rollback().await.map_err(map_error),
            SqlxSession::Conn(_) => Ok(()),
        }
    }

    pub fn backend<'h>(&'h mut self) -> BackendImpl<'h, 'c, DB> {
        BackendImpl { handle: self }
    }
}

impl<'h, 'c, DB> sqlx::Executor<'h> for BackendImpl<'h, 'c, DB>
where
    DB: sqlx::Database,
    for<'e> &'e mut DB::Connection: sqlx::Executor<'e, Database = DB>,
{
    type Database = DB;

    /// Execute multiple queries and return the generated results as a stream
    /// from each query, in a stream.
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
        match self.handle {
            SqlxSession::Pool(pool) => pool.fetch_many(query),
            SqlxSession::Tx(tx) => tx.fetch_many(query),
            SqlxSession::Conn(conn) => conn.fetch_many(query),
        }
    }

    /// Execute the query and returns at most one row.
    fn fetch_optional<'e, 'q: 'e, E>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<<Self::Database as sqlx::Database>::Row>, sqlx::Error>>
    where
        'c: 'e,
        'h: 'e,
        E: 'q + sqlx::Execute<'q, Self::Database>,
    {
        match self.handle {
            SqlxSession::Pool(pool) => pool.fetch_optional(query),
            SqlxSession::Tx(tx) => tx.fetch_optional(query),
            SqlxSession::Conn(conn) => conn.fetch_optional(query),
        }
    }

    /// Prepare the SQL query, with parameter type information, to inspect the
    /// type information about its parameters and results.
    ///
    /// Only some database drivers (PostgreSQL, MSSQL) can take advantage of
    /// this extra information to influence parameter type inference.
    fn prepare_with<'e>(
        self,
        sql: SqlStr,
        parameters: &'e [<Self::Database as sqlx::Database>::TypeInfo],
    ) -> BoxFuture<'e, Result<<Self::Database as sqlx::Database>::Statement, sqlx::Error>>
    where
        'c: 'e,
        'h: 'e,
    {
        match self.handle {
            SqlxSession::Pool(pool) => pool.prepare_with(sql, parameters),
            SqlxSession::Tx(tx) => tx.prepare_with(sql, parameters),
            SqlxSession::Conn(conn) => conn.prepare_with(sql, parameters),
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
        match self.handle {
            SqlxSession::Pool(pool) => pool.describe(sql),
            SqlxSession::Tx(tx) => tx.describe(sql),
            SqlxSession::Conn(conn) => conn.describe(sql),
        }
    }
}
