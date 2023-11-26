alter table server
add column environment text not null default 'EnvProd';

alter table project
add column has_dev text not null default 'False';

alter table project
add column has_uat text not null default 'False';

alter table project
add column has_stage text not null default 'False';

alter table project
add column has_prod text not null default 'True';
