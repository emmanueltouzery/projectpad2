alter table project_note rename to temp_project_note;

CREATE TABLE project_note (
id INTEGER PRIMARY KEY,
title TEXT NOT NULL,
contents TEXT NOT NULL,
has_dev   INTEGER not null,
has_uat   INTEGER not null,
has_stage INTEGER not null,
has_prod  INTEGER not null,
group_name TEXT CHECK(LENGTH(group_name) > 0),
project_id INTEGER NOT NULL,
FOREIGN KEY(project_id) REFERENCES project(id) ON DELETE CASCADE);


insert into project_note
select id, title, contents,
1, 1, 1, 1, group_name, project_id
from temp_project_note;

drop table temp_project_note;
