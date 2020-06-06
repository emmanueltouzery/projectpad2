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
