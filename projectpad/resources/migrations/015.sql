alter table server
add column ssh_tunnel_port INTEGER;

alter table server
add column ssh_tunnel_through_server_id INTEGER references server(id);

CREATE UNIQUE INDEX IF NOT EXISTS server_ssh_tunnel_port ON server (ssh_tunnel_port);
