//! Server functions for up/down-voting individual recordings, backed by SQLite.

use dioxus::prelude::*;

#[cfg(feature = "server")]
const SOUND_RATINGS_DB: &str =
    "/Users/diegoesparza/CS_Ventures/current_projects/bird-call-matching/sound-ratings.db";

#[cfg(feature = "server")]
fn to_server_error(e: rusqlite::Error) -> ServerFnError {
    ServerFnError::new(e.to_string())
}

#[cfg(feature = "server")]
fn open_db() -> Result<rusqlite::Connection, ServerFnError> {
    rusqlite::Connection::open(SOUND_RATINGS_DB).map_err(to_server_error)
}

#[server]
pub async fn upvote_sound(id: String) -> Result<(), ServerFnError> {
    open_db()?
        .execute(
            "INSERT INTO sound_ratings (id, upvotes, downvotes) VALUES (?1, 1, 0)
             ON CONFLICT(id) DO UPDATE SET upvotes = upvotes + 1",
            (id,),
        )
        .map_err(to_server_error)?;

    Ok(())
}

#[server]
pub async fn downvote_sound(id: String) -> Result<(), ServerFnError> {
    open_db()?
        .execute(
            "INSERT INTO sound_ratings (id, upvotes, downvotes) VALUES (?1, 0, 1)
             ON CONFLICT(id) DO UPDATE SET downvotes = downvotes + 1",
            (id,),
        )
        .map_err(to_server_error)?;

    Ok(())
}

#[server]
pub async fn get_ratings(id: String) -> Result<(i64, i64), ServerFnError> {
    let result = open_db()?.query_row(
        "SELECT upvotes, downvotes FROM sound_ratings WHERE id = ?1",
        (id,),
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok(ratings) => Ok(ratings),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok((0, 0)),
        Err(e) => Err(to_server_error(e)),
    }
}
