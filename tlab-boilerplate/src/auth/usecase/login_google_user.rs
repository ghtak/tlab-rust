use std::sync::Arc;

use crate::{
    app_container::AppDB,
    auth::{
        entity::{Provider, UserStatus},
        repository::user_repository,
        service::TokenService,
        usecase::{CreateSocialUserCommand, CreateSocialUserUsecase},
    },
};

pub struct LoginGoogleUserCommand {
    pub code: String,
    pub state: Option<String>,
}

pub struct LoginGoogleUserUsecase {
    app_db: Arc<AppDB>,
    token_service: Arc<TokenService>,
    google_oauth2: tlab::google_oauth2::Config,
}

impl LoginGoogleUserUsecase {
    pub fn new(
        app_db: Arc<AppDB>,
        token_service: Arc<TokenService>,
        google_oauth2: tlab::google_oauth2::Config,
    ) -> Self {
        Self {
            app_db,
            token_service,
            google_oauth2,
        }
    }

    pub async fn execute(
        &self,
        command: &LoginGoogleUserCommand,
    ) -> tlab::Result<tlab::jwt::TokenPair> {
        if !verify_oauth2_state(command.state.as_deref()) {
            return Err(tlab::Error::InvalidToken);
        }

        let claims =
            tlab::google_oauth2::handle_oauth2_callback(&self.google_oauth2, &command.code).await?;
        self.login_with_claims(claims).await
    }

    async fn login_with_claims(
        &self,
        claims: tlab::google_oauth2::GoogleIdTokenClaims,
    ) -> tlab::Result<tlab::jwt::TokenPair> {
        let mut conn = self.app_db.conn().await?;
        let identity =
            user_repository::find_user_identity(&mut conn.context(), Provider::Google, &claims.sub)
                .await?;
        let account = if let Some(identity) = identity {
            let account =
                user_repository::find_user_account(&mut conn.context(), identity.user_account_id)
                    .await?
                    .ok_or_else(|| {
                        tlab::Error::IllegalState("social user account not found".into())
                    })?;
            drop(conn);
            account
        } else {
            drop(conn);
            let email = claims
                .email
                .filter(|email| !email.is_empty())
                .ok_or(tlab::Error::InvalidCredentials)?;
            let command = CreateSocialUserCommand {
                name: claims
                    .name
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| email.clone()),
                email,
                provider: Provider::Google,
                provider_subject: claims.sub,
            };
            CreateSocialUserUsecase::new(self.app_db.clone())
                .execute(&command)
                .await?
        };

        if account.status != UserStatus::Active {
            return Err(tlab::Error::InvalidCredentials);
        }

        let mut conn = self.app_db.conn().await?;
        self.token_service
            .issue_login_tokens(&mut conn.context(), account.id)
            .await
    }
}

fn verify_oauth2_state(state: Option<&str>) -> bool {
    // TODO: Bind the state to the authorization request and verify it here.
    state.is_some_and(|value| uuid::Uuid::parse_str(value).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth::repository::refresh_token_repository, test_app, test_db};
    use tlab::jwt::TokenUse;

    #[tokio::test]
    async fn logs_in_google_user_and_reuses_existing_account() {
        let (_, app) = test_app::setup().await;
        let usecase = LoginGoogleUserUsecase::new(
            app.database.clone(),
            app.token_service.clone(),
            app.config.google_oauth2.clone(),
        );
        let email = test_db::unique_email("google-login");
        let subject = uuid::Uuid::new_v4().to_string();

        for _ in 0..2 {
            let tokens = usecase
                .login_with_claims(tlab::google_oauth2::GoogleIdTokenClaims {
                    sub: subject.clone(),
                    email: Some(email.clone()),
                    name: Some("Alice".into()),
                })
                .await
                .unwrap();
            let claims = app.jwt_codec.verify(&tokens.access.token).unwrap();
            assert_eq!(claims.token_use, TokenUse::Access);
            let app_claims: crate::auth::access_claims::AppClaims =
                serde_json::from_value(claims.app.unwrap()).unwrap();
            assert!(app_claims.role_ids.is_empty());
        }

        let mut conn = app.database.conn().await.unwrap();
        let identity =
            user_repository::find_user_identity(&mut conn.context(), Provider::Google, &subject)
                .await
                .unwrap()
                .unwrap();
        let sessions = refresh_token_repository::find_all_by_user_account_id(
            &mut conn.context(),
            identity.user_account_id,
        )
        .await
        .unwrap();
        assert_eq!(sessions.len(), 2);
        drop(conn);

        let mut tx = app.database.tx().await.unwrap();
        for session in sessions {
            refresh_token_repository::delete(
                &mut tx.context(),
                identity.user_account_id,
                session.session_id,
            )
            .await
            .unwrap();
        }
        user_repository::delete_user_identity(&mut tx.context(), identity.id)
            .await
            .unwrap();
        user_repository::delete_user_account(&mut tx.context(), identity.user_account_id)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
}
