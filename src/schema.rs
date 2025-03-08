// @generated automatically by Diesel CLI.

diesel::table! {
    fingerprints (id) {
        id -> Nullable<Integer>,
        song_id -> Integer,
        hash -> Integer,
        time_offset -> Float,
        confidence -> Float,
    }
}

diesel::table! {
    songs (id) {
        id -> Nullable<Integer>,
        title -> Text,
        artist -> Nullable<Text>,
        album -> Nullable<Text>,
        duration -> Nullable<Float>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(fingerprints -> songs (song_id));

diesel::allow_tables_to_appear_in_same_query!(
    fingerprints,
    songs,
);
