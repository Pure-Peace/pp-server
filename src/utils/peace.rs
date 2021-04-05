#![allow(unused_macros)]

use actix_web::HttpRequest;
use chrono::{DateTime, Local};
use serde::de::{self, Deserialize, Deserializer};
use std::fmt::Display;
use std::str::FromStr;
use tokio_pg_mapper::FromTokioPostgresRow;

use crate::database::Database;

#[inline(always)]
pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

#[inline(always)]
pub fn noew_time_local() -> DateTime<Local> {
    Local::now()
}

#[inline(always)]
/// Get real ip from request
pub async fn get_realip(req: &HttpRequest) -> Result<String, ()> {
    match req.connection_info().realip_remote_addr() {
        Some(ip) => Ok(match ip.find(':') {
            Some(idx) => ip[0..idx].to_string(),
            None => ip.to_string(),
        }),
        None => Err(()),
    }
}

#[inline(always)]
/// Get osu token from headers
pub async fn get_token(req: &HttpRequest) -> String {
    match req.headers().get("osu-token") {
        Some(version) => version.to_str().unwrap_or("unknown").to_string(),
        None => "unknown".to_string(),
    }
}

/// Get beatmap ratings from database
#[inline(always)]
pub async fn get_beatmap_rating(beatmap_md5: &String, database: &Database) -> Option<f32> {
    match database
        .pg
        .query_first(
            r#"SELECT AVG("rating")::float4 FROM "beatmaps"."ratings" WHERE "map_md5" = $1"#,
            &[beatmap_md5],
        )
        .await
    {
        Ok(value) => Some(value.get(0)),
        Err(err) => {
            error!(
                "failed to get avg rating from beatmap {}, err: {:?}",
                beatmap_md5, err
            );
            None
        }
    }
}

#[inline(always)]
fn get_type_of<T>(_: &T) -> String {
    format!("{}", std::any::type_name::<T>())
}

#[inline(always)]
/// Utils for struct from database
pub async fn struct_from_database<T: FromTokioPostgresRow>(
    table: &str,
    schema: &str,
    query_by: &str,
    fields: &str,
    param: &(dyn tokio_postgres::types::ToSql + Sync),
    database: &Database,
) -> Option<T> {
    let type_name = std::any::type_name::<T>();
    let query = format!(
        "SELECT {} FROM \"{}\".\"{}\" WHERE \"{}\" = $1;",
        fields, table, schema, query_by
    );
    debug!("struct_from_database query: {}", query);
    let row = database.pg.query_first(&query, &[param]).await;
    if let Err(err) = row {
        debug!(
            "Failed to get {} {:?} from database table {}.{} error: {:?}",
            type_name, param, table, schema, err
        );
        return None;
    }

    let row = row.unwrap();
    match <T>::from_row(row) {
        Ok(result) => Some(result),
        Err(err) => {
            error!(
                "Failed to deserialize {} from pg-row! error: {:?}",
                type_name, err
            );
            None
        }
    }
}

#[inline(always)]
pub fn build_s(len: usize) -> String {
    let mut s = String::new();
    for i in 1..len + 1 {
        s += (if i == len {
            format!("${}", i)
        } else {
            format!("${},", i)
        })
        .as_str();
    }
    s
}
