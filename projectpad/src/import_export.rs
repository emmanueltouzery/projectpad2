use diesel::prelude::*;
use projectpadsql::models::{
    EnvironmentType, Project, Server, ServerDatabase, ServerExtraUserAccount, ServerNote,
    ServerWebsite,
};

pub fn export_project(sql_conn: &diesel::SqliteConnection, project: &Project) {
    let mut output = format!("# {}\n", project.name);

    let group_names = projectpadsql::get_project_group_names(sql_conn, project.id);

    // if I export a 7zip i can export project icons and attachments in the zip too...
    if project.has_dev {
        export_env(
            sql_conn,
            project,
            EnvironmentType::EnvDevelopment,
            &group_names,
            &mut output,
        );
    }
    if project.has_stage {
        export_env(
            sql_conn,
            project,
            EnvironmentType::EnvStage,
            &group_names,
            &mut output,
        );
    }
    if project.has_uat {
        export_env(
            sql_conn,
            project,
            EnvironmentType::EnvUat,
            &group_names,
            &mut output,
        );
    }
    if project.has_prod {
        export_env(
            sql_conn,
            project,
            EnvironmentType::EnvProd,
            &group_names,
            &mut output,
        );
    }
    println!("{}", &output);
}

fn export_env(
    sql_conn: &diesel::SqliteConnection,
    project: &Project,
    env: EnvironmentType,
    group_names: &[String],
    output: &mut String,
) {
    use projectpadsql::schema::server::dsl as srv;
    output.push_str(&format!("## {}\n", env));

    let srvs = srv::server
        .filter(
            srv::project_id
                .eq(project.id)
                .and(srv::environment.eq(env))
                .and(srv::group_name.is_null()),
        )
        .order((srv::group_name.asc(), srv::desc.asc()))
        .load::<Server>(sql_conn)
        .unwrap();

    for srv in srvs {
        export_server(sql_conn, &srv, output);
    }

    // project notes

    // server links

    // project POIs

    for group_name in group_names {
        output.push_str(&format!("### {}\n", group_name));
        let srvs = srv::server
            .filter(
                srv::project_id
                    .eq(project.id)
                    .and(srv::environment.eq(env))
                    .and(srv::group_name.eq(group_name)),
            )
            .order((srv::group_name.asc(), srv::desc.asc()))
            .load::<Server>(sql_conn)
            .unwrap();

        for srv in srvs {
            export_server(sql_conn, &srv, output);
        }

        // project notes

        // server links

        // project POIs
    }
    output.push_str("\n");
}

trait ImportExport<T> {
    // fn import<'a, 'b>(lines: &'a [&'b str]) -> (Result<(), String>, &'a [&'b str]);
    fn export(&self, output: &mut String);
}

macro_rules! generate_importexport {
    // ####### i don't want to generate the object itself, but the stuff for the insert for diesel!!!
    ($type: ident, $( $field:tt, $field_name:expr ),+ ) => {
        impl ImportExport<$type> for $type {
            // fn import<'a, 'b>(lines: &'a [&'b str]) -> (Result<$type, String>, &'a [&'b str]) {
            //     (Ok($type {
            //         $({
            //             $field: "",
            //         })+
            //     }), lines)
            // }
            fn export(&self, output: &mut String) {
                $({
                    if !self.$field.to_string().is_empty() {
                        output.push_str(&format!("{}: {}\n", $field_name, self.$field));
                    }
                })+
            }
        }
    };
}

generate_importexport!(
    Server,
    desc,
    "Description",
    ip,
    "Address",
    text,
    "Text",
    is_retired,
    "Is retired",
    username,
    "Username",
    password,
    "Password",
    server_type,
    "Server type",
    access_type,
    "Access type"
);

generate_importexport!(
    ServerWebsite,
    desc,
    "Description",
    url,
    "Address",
    text,
    "Text",
    username,
    "Username",
    password,
    "Password"
);

generate_importexport!(
    ServerDatabase,
    desc,
    "Description",
    name,
    "Name",
    text,
    "Text",
    username,
    "Username",
    password,
    "Password"
);

generate_importexport!(
    ServerExtraUserAccount,
    username,
    "Username",
    password,
    "Password",
    desc,
    "Description"
);

generate_importexport!(ServerNote, title, "Title", contents, "Content");

fn export_server(sql_conn: &diesel::SqliteConnection, server: &Server, output: &mut String) {
    use projectpadsql::schema::server_database::dsl as srv_db;
    use projectpadsql::schema::server_extra_user_account::dsl as srv_usr;
    use projectpadsql::schema::server_note::dsl as srv_note;
    use projectpadsql::schema::server_website::dsl as srv_www;
    output.push_str("---\n#### Server\n");
    // output.push_str(&format!("#### Server: {}\n", server.desc));
    server.export(output);

    // server websites
    let server_websites = srv_www::server_website
        .filter(srv_www::server_id.eq(server.id))
        .order(srv_www::desc.asc())
        .load::<ServerWebsite>(sql_conn)
        .unwrap();
    for server_website in server_websites {
        output.push_str("\n##### Server website\n");
        server_website.export(output);
    }

    let server_dbs = srv_db::server_database
        .filter(srv_db::server_id.eq(server.id))
        .order(srv_db::desc.asc())
        .load::<ServerDatabase>(sql_conn)
        .unwrap();
    for server_db in server_dbs {
        output.push_str("\n##### Server database\n");
        server_db.export(output);
    }

    let server_notes = srv_note::server_note
        .filter(srv_note::server_id.eq(server.id))
        .order(srv_note::title.asc())
        .load::<ServerNote>(sql_conn)
        .unwrap();
    for server_note in server_notes {
        output.push_str("\n##### Server note\n");
        server_note.export(output);
    }

    let server_users = srv_usr::server_extra_user_account
        .filter(srv_usr::server_id.eq(server.id))
        .order(srv_usr::username.asc())
        .load::<ServerExtraUserAccount>(sql_conn)
        .unwrap();
    for server_user in server_users {
        output.push_str("\n##### Server extra user\n");
        server_user.export(output);
    }
}
