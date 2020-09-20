alter table server_point_of_interest
add column group_name TEXT CHECK(LENGTH(group_name) > 0);

alter table server_website
add column group_name TEXT CHECK(LENGTH(group_name) > 0);

alter table server_database
add column group_name TEXT CHECK(LENGTH(group_name) > 0);

alter table server_extra_user_account
add column group_name TEXT CHECK(LENGTH(group_name) > 0);
