create table project_note (id INTEGER PRIMARY KEY,
 title TEXT NOT NULL,
 contents TEXT NOT NULL,
 group_name TEXT CHECK(LENGTH(group_name) > 0),
 project_id INTEGER NOT NULL,
 FOREIGN KEY(project_id) REFERENCES project(id) ON DELETE CASCADE);
