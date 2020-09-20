create table server_note (id INTEGER PRIMARY KEY,
 title TEXT NOT NULL,
 contents TEXT NOT NULL,
 group_name TEXT CHECK(LENGTH(group_name) > 0),
 server_id INTEGER NOT NULL,
 FOREIGN KEY(server_id) REFERENCES server(id) ON DELETE CASCADE);
