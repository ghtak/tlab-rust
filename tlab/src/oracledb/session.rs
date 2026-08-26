use std::sync::atomic::{AtomicU64, Ordering};

use crate::oracledb::map_error;

pub struct Session<'c> {
    connection: ConnectionHolder<'c>,
    scope: Scope,
}

enum ConnectionHolder<'c> {
    Owned(Connection),
    Borrowed(&'c mut Connection),
}

struct Connection {
    inner: deadpool_oracle::Object,
    next_savepoint: AtomicU64,
}

enum Scope {
    Session,
    Root,
    Savepoint(String),
}

impl Session<'static> {
    pub(crate) fn new(inner: deadpool_oracle::Object) -> Self {
        Self {
            connection: ConnectionHolder::Owned(Connection {
                inner,
                next_savepoint: AtomicU64::new(0),
            }),
            scope: Scope::Session,
        }
    }
}

impl Session<'_> {
    pub fn backend(&mut self) -> &mut oracle_rs::Connection {
        &mut self.connection_mut().inner
    }

    pub async fn begin(&mut self) -> crate::Result<Session<'_>> {
        let scope = match self.scope {
            Scope::Session => Scope::Root,
            Scope::Root | Scope::Savepoint(_) => {
                let connection = self.connection_mut();
                let sequence = connection.next_savepoint.fetch_add(1, Ordering::Relaxed);
                let savepoint = format!("tlab_savepoint_{sequence}");
                connection
                    .inner
                    .savepoint(&savepoint)
                    .await
                    .map_err(map_error)?;
                Scope::Savepoint(savepoint)
            }
        };

        Ok(Session {
            connection: ConnectionHolder::Borrowed(self.connection_mut()),
            scope,
        })
    }

    pub async fn commit(self) -> crate::Result<()> {
        if matches!(self.scope, Scope::Root) {
            self.connection().inner.commit().await.map_err(map_error)?;
        }
        Ok(())
    }

    pub async fn rollback(self) -> crate::Result<()> {
        match self.scope {
            Scope::Session => {}
            Scope::Root => self
                .connection()
                .inner
                .rollback()
                .await
                .map_err(map_error)?,
            Scope::Savepoint(ref savepoint) => self
                .connection()
                .inner
                .rollback_to_savepoint(&savepoint)
                .await
                .map_err(map_error)?,
        }
        Ok(())
    }

    fn connection(&self) -> &Connection {
        match &self.connection {
            ConnectionHolder::Owned(connection) => connection,
            ConnectionHolder::Borrowed(connection) => connection,
        }
    }

    fn connection_mut(&mut self) -> &mut Connection {
        match &mut self.connection {
            ConnectionHolder::Owned(connection) => connection,
            ConnectionHolder::Borrowed(connection) => connection,
        }
    }
}
