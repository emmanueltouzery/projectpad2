alter table server
add column group_name TEXT CHECK(LENGTH(group_name) > 0);

alter table project_point_of_interest
add column group_name TEXT CHECK(LENGTH(group_name) > 0);
