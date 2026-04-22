// Copyright 2024 - developers of the `tdlib-rs` project.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Strongly-typed newtypes for TDLib identifier fields.
//!
//! Each newtype wraps a raw integer with `#[repr(transparent)]` +
//! `#[serde(transparent)]` so the wire format is unchanged and the memory
//! layout matches the underlying `i64` / `i32`.
//!
//! **No escape hatches.** These types deliberately do not implement
//! `From<i64>` / `From<i32>` or expose an accessor to the raw inner value.
//! Construction is possible only via:
//!
//! * `Deserialize` — for JSON → Rust on the TDLib wire boundary (works
//!   against the private inner field because the derived impl lives in the
//!   same module).
//! * `FromSql<_, Turso>` (with the `diesel` feature) — for DB reads.
//! * `Default` — for `ChatId::default() == ChatId(0)` etc., required so
//!   the codegen-emitted `#[derive(Default)]` on enclosing structs still
//!   compiles.
//!
//! Code that needs to act on the underlying integer must add a domain method
//! to the newtype (e.g. `is_group_or_channel`), not a raw accessor. This
//! pushes callers to name the operation, which is the whole point of having
//! the newtype in the first place.
//!
//! The set of wrapped fields is curated in `tdlib-rs-gen/src/rustifier.rs`
//! — the codegen consults the mapping when emitting struct fields and
//! function arguments.

use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(feature = "diesel")]
use diesel::deserialize::{self, FromSql, FromSqlRow};
#[cfg(feature = "diesel")]
use diesel::expression::AsExpression;
#[cfg(feature = "diesel")]
use diesel::serialize::{self, Output, ToSql};
#[cfg(feature = "diesel")]
use diesel::sql_types::{BigInt, Integer};
#[cfg(feature = "diesel")]
use turbo_diesel::union::{DecodeError, FieldCodec};
#[cfg(feature = "diesel")]
use turbo_diesel::Turso;

macro_rules! int53_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Default,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
        )]
        #[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
        #[repr(transparent)]
        #[serde(transparent)]
        #[cfg_attr(feature = "diesel", diesel(sql_type = BigInt))]
        pub struct $name(i64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        #[cfg(feature = "diesel")]
        impl ToSql<BigInt, Turso> for $name {
            fn to_sql<'b>(
                &'b self,
                out: &mut Output<'b, '_, Turso>,
            ) -> serialize::Result {
                <i64 as ToSql<BigInt, Turso>>::to_sql(&self.0, out)
            }
        }

        #[cfg(feature = "diesel")]
        impl FromSql<BigInt, Turso> for $name {
            fn from_sql(
                bytes: <Turso as diesel::backend::Backend>::RawValue<'_>,
            ) -> deserialize::Result<Self> {
                <i64 as FromSql<BigInt, Turso>>::from_sql(bytes).map(Self)
            }
        }

        /// `FieldCodec` lets this newtype appear as a field inside a
        /// `#[derive(UnionSchema)]` STRUCT variant (e.g. a `MessageId`
        /// UNION that embeds `ChatId` inside its Telegram variant).
        #[cfg(feature = "diesel")]
        impl FieldCodec for $name {
            fn sql_type() -> &'static str {
                "INT"
            }
            fn into_value(self) -> turso::Value {
                turso::Value::Integer(self.0)
            }
            fn from_value(v: turso::Value) -> Result<Self, DecodeError> {
                <i64 as FieldCodec>::from_value(v).map(Self)
            }
        }
    };
}

macro_rules! int32_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Default,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
        )]
        #[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
        #[repr(transparent)]
        #[serde(transparent)]
        #[cfg_attr(feature = "diesel", diesel(sql_type = Integer))]
        pub struct $name(i32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        #[cfg(feature = "diesel")]
        impl ToSql<Integer, Turso> for $name {
            fn to_sql<'b>(
                &'b self,
                out: &mut Output<'b, '_, Turso>,
            ) -> serialize::Result {
                <i32 as ToSql<Integer, Turso>>::to_sql(&self.0, out)
            }
        }

        #[cfg(feature = "diesel")]
        impl FromSql<Integer, Turso> for $name {
            fn from_sql(
                bytes: <Turso as diesel::backend::Backend>::RawValue<'_>,
            ) -> deserialize::Result<Self> {
                <i32 as FromSql<Integer, Turso>>::from_sql(bytes).map(Self)
            }
        }

        /// `FieldCodec` lets this newtype appear as a field inside a
        /// `#[derive(UnionSchema)]` STRUCT variant (e.g. a `ChatType`
        /// UNION that embeds `SecretChatId` inside its secret variant).
        #[cfg(feature = "diesel")]
        impl FieldCodec for $name {
            fn sql_type() -> &'static str {
                "INT"
            }
            fn into_value(self) -> turso::Value {
                turso::Value::Integer(self.0 as i64)
            }
            fn from_value(v: turso::Value) -> Result<Self, DecodeError> {
                <i32 as FieldCodec>::from_value(v).map(Self)
            }
        }
    };
}

int53_newtype! {
    /// Telegram chat identifier (`int53`). Unified across private chats,
    /// basic groups, supergroups, channels, and secret chats — always the
    /// value you pass to chat-scoped TDLib functions like `sendMessage`.
    ChatId
}

impl ChatId {
    /// Whether this `ChatId` refers to a private 1:1 chat with a user.
    /// Per Telegram's sign convention, user chats have positive `chat_id`
    /// values. Doesn't allocate or consult any external data.
    pub const fn is_user(&self) -> bool {
        self.0 > 0
    }

    /// Whether this `ChatId` refers to a group, supergroup, or channel
    /// (non-user chat). Per Telegram's sign convention these have a
    /// negative `chat_id`. Non-positive includes `0`, which TDLib does not
    /// use as a real chat id; callers that need to exclude the sentinel
    /// should check `chat_id != ChatId::default()` separately.
    pub const fn is_chat(&self) -> bool {
        self.0 < 0
    }

    /// **DO NOT CALL DIRECTLY.** The only intended caller is the
    /// `IntoTelegramId for ChatId` impl in fifteen-db, which needs to
    /// synthesize a `UserId` from a positive-signed `ChatId` to build
    /// `SocialId::TelegramUser(UserId)` from the otherwise-opaque wire
    /// value. Any other caller wants either a real `UserId` (pull it
    /// from `Chat.type = ChatTypePrivate { user_id }`) or the
    /// `SocialId::telegram(chat_id)` generic constructor — never this.
    ///
    /// The screamy name is intentional: it makes any stray adoption
    /// trivial to catch in grep / code review.
    #[allow(non_snake_case)]
    pub const fn DO_NOT_USE___as_user_id(&self) -> Option<UserId> {
        if self.is_user() {
            Some(UserId(self.0))
        } else {
            None
        }
    }
}

int53_newtype! {
    /// Telegram user identifier (`int53`). Shares the `int53` number space
    /// with `ChatId` for 1:1 DMs (a private chat's `ChatId` equals the
    /// partner's `UserId`), but the types are kept distinct so accidentally
    /// passing a group chat_id where a user_id is expected is a compile error.
    UserId
}

int53_newtype! {
    /// Telegram message identifier (`int53`). Unique within a chat; not
    /// globally unique. Pair with the owning `ChatId` to address a message.
    MessageId
}

int53_newtype! {
    /// Telegram forum-topic identifier (`int53`). Within a forum chat, each
    /// topic has its own monotonic message id space.
    TopicId
}

int32_newtype! {
    /// Telegram file identifier (`int32`). References a cached file inside
    /// TDLib's local cache; not stable across sessions.
    FileId
}

int32_newtype! {
    /// Secret-chat identifier (`int32`). Distinct from `ChatId` — refers to
    /// the underlying secret-chat object rather than the computed `ChatId`
    /// that TDLib exposes for that chat via the unified chat API.
    SecretChatId
}
