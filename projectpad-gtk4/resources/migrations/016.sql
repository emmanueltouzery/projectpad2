CREATE TABLE server_link (id INTEGER PRIMARY KEY,
       desc TEXT NOT NULL,
	     linked_server_id INTEGER NOT NULL,
       environment TEXT NOT NULL,
       group_name TEXT CHECK(LENGTH(group_name) > 0),
	     project_id INTEGER NOT NULL,
	     FOREIGN KEY(linked_server_id) REFERENCES server(id) ON DELETE CASCADE,
	     FOREIGN KEY(project_id) REFERENCES project(id) ON DELETE CASCADE);
