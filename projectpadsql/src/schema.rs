use diesel::prelude::*;

// https://gitter.im/diesel-rs/diesel?at=5d420302b0bf183ea3785273
table! {
    projects {
        id -> Int4,
        name -> Varchar,
        icon -> Binary,
        has_dev -> Bool,
        has_uat -> Bool,
        has_stage -> Bool,
        has_prod -> Bool,
    }
}
