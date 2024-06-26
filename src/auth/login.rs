use actix_jwt_auth_middleware::FromRequest;
use actix_web::{cookie::Cookie, post, web, HttpResponse, Responder};
use chrono::Duration;
use jwt_compact::{
    alg::{Hs256, Hs256Key},
    AlgorithmExt, Claims, Header, TimeOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{schema::users::dsl::*, DbPool};
use diesel::prelude::*;

use super::{register::password_hash, users::UserLoginWeb};

lazy_static::lazy_static! {
    pub static ref JWT_ENCODING_KEY : Hs256Key = Hs256Key::from(dotenvy::var(
        "JWT_ENCODING_KEY"
    ).expect("MISSING JWT TOKEN ENCODER").as_bytes());
}

#[post("/login")]
pub async fn login(
    pool: web::Data<DbPool>,
    creds: web::Json<UserLoginWeb>,
) -> actix_web::Result<impl Responder> {
    login_internal(pool, creds.0).await
}

pub(crate) async fn login_internal(
    pool: web::Data<DbPool>,
    creds: UserLoginWeb,
) -> actix_web::Result<HttpResponse> {
    let uname = web::block(move || {
        let hash = password_hash(&creds.password);
        let mut conn = pool.get().expect("couldn't get db connection from pool");

        users
            .filter(email.eq(&creds.email).and(pass_hash.eq(&hash)))
            .select((username, id))
            .first::<(String, i32)>(&mut conn)
    })
    .await?;

    let Ok((uname, uid)) = uname else {
        return Ok(HttpResponse::NotFound().json("Wrong credentials"));
    };

    Ok(jwt_response(uname, uid))
}


#[derive(Debug, Serialize, Deserialize, Clone, FromRequest)]
pub struct UserClaims {
    pub username: String,
    pub id: i32,
}


pub fn jwt_response(uname: String, uid: i32) -> HttpResponse {
    // get current time in unix seconds

    let jwt = Hs256
        .token(
            &Header::empty(),
            &Claims::new(UserClaims {
                username: uname.clone(),
                id: uid,
            })
            .set_duration(&TimeOptions::default(), Duration::days(7)),
            &*JWT_ENCODING_KEY,
        )
        .unwrap();

    let jwt_cookie = Cookie::build("jwt_token", jwt)
        .domain("localhost")
        .path("/")
        // .secure(true)
        .http_only(true)
        .finish();

    HttpResponse::Ok().cookie(jwt_cookie).json(json!( {
        "username": uname
    }))
}
