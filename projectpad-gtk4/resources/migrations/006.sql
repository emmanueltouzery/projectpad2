alter table server
add column auth_key BLOB;

alter table server
add column auth_key_filename TEXT;
