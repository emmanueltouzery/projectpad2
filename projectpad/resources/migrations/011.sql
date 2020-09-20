CREATE TABLE server_extra_user_account (id INTEGER PRIMARY KEY,
       username TEXT NOT NULL,
       password TEXT NOT NULL,
       desc TEXT NOT NULL,
       auth_key BLOB,
       auth_key_filename TEXT,
       server_id INTEGER NOT NULL,
       FOREIGN KEY(server_id) REFERENCES server(id) ON DELETE CASCADE);
