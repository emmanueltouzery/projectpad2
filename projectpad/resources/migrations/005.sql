alter table server_website
add column server_database_id integer null references server_database(id);
