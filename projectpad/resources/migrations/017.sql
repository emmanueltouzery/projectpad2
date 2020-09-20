alter table project rename to temp_project;

CREATE TABLE project (
       id INTEGER PRIMARY KEY,
       name TEXT NOT NULL COLLATE NOCASE,
       icon BLOB NOT NULL,
       has_dev   INTEGER not null default 0,
       has_uat   INTEGER not null default 0,
       has_stage INTEGER not null default 0,
       has_prod  INTEGER not null default 0);

insert into project
  select id, name, icon,
  case has_dev   when 'True' then 1 else 0 end,
  case has_uat   when 'True' then 1 else 0 end,
  case has_stage when 'True' then 1 else 0 end,
  case has_prod  when 'True' then 1 else 0 end
from temp_project;

drop table temp_project;
