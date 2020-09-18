use diesel::prelude::*;

// https://gitter.im/diesel-rs/diesel?at=5d420302b0bf183ea3785273
table! {
    project {
        id -> Integer,
        name -> Varchar,
        icon -> Nullable<Binary>,
        has_dev -> Bool,
        has_uat -> Bool,
        has_stage -> Bool,
        has_prod -> Bool,
    }
}

table! {
    server {
        id -> Integer,
        desc -> Varchar,
        ip -> Varchar,
        text -> Varchar,
        is_retired -> Bool,
        username -> Varchar,
        password -> Varchar,
        auth_key -> Nullable<Binary>,
        auth_key_filename -> Nullable<Varchar>,
        #[sql_name="type"]
        server_type -> Varchar,
        access_type -> Varchar,
        ssh_tunnel_port -> Nullable<Integer>,
        ssh_tunnel_through_server_id -> Nullable<Integer>,
        environment -> Varchar,
        group_name -> Nullable<Varchar>,
        project_id -> Integer,
    }
}

table! {
    project_note {
        id -> Integer,
        title -> Varchar,
        contents -> Varchar,
        has_dev -> Bool,
        has_uat -> Bool,
        has_stage -> Bool,
        has_prod -> Bool,
        group_name -> Nullable<Varchar>,
        project_id -> Integer,
    }
}

table! {
    project_point_of_interest {
        id -> Integer,
        desc -> Varchar,
        path -> Varchar,
        text -> Varchar,
        interest_type -> Varchar,
        group_name -> Nullable<Varchar>,
        project_id -> Integer,
    }
}

table! {
    server_link {
        id -> Integer,
        desc -> Varchar,
        linked_server_id -> Integer,
        environment -> Varchar,
        group_name -> Nullable<Varchar>,
        project_id -> Integer,
    }
}

table! {
    server_website {
        id -> Integer,
        desc -> Varchar,
        url -> Varchar,
        text -> Varchar,
        username -> Varchar,
        password -> Varchar,
        server_database_id -> Nullable<Integer>,
        group_name -> Nullable<Varchar>,
        server_id -> Integer,
    }
}

table! {
    server_point_of_interest {
        id -> Integer,
        desc -> Varchar,
        path -> Varchar,
        text -> Varchar,
        interest_type -> Varchar,
        run_on -> Varchar,
        group_name -> Nullable<Varchar>,
        server_id -> Integer,
    }
}

table! {
    server_note {
        id -> Integer,
        title -> Varchar,
        contents -> Varchar,
        group_name -> Nullable<Varchar>,
        server_id -> Integer,
    }
}

table! {
    server_extra_user_account {
        id -> Integer,
        username -> Varchar,
        password -> Varchar,
        desc -> Varchar,
        auth_key -> Nullable<Binary>,
        auth_key_filename -> Nullable<Varchar>,
        group_name -> Nullable<Varchar>,
        server_id -> Integer,
    }
}

table! {
    server_database {
        id -> Integer,
        desc -> Varchar,
        name -> Varchar,
        text -> Varchar,
        username -> Varchar,
        password -> Varchar,
        group_name -> Nullable<Varchar>,
        server_id -> Integer,
    }
}

joinable!(server_website -> server_database (server_database_id));
allow_tables_to_appear_in_same_query!(server_website, server_database, server);
joinable!(server_website -> server (server_id));

joinable!(server_link -> server (linked_server_id));
allow_tables_to_appear_in_same_query!(server_link, server);

joinable!(server_database -> server (server_id));

joinable!(server -> project (project_id));
allow_tables_to_appear_in_same_query!(project, server);
