# Projectpad

## Purpose

Projectpad allows to manage secret credentials and server information that you need to handle as a software developer. List of servers, list of point of interests on those servers (applications, log files, databases, servers). It will securely store passwords and keys. It will also allow you to run commands (locally or on SSH servers), open terminals on remote SSH servers and open windows remote desktop sessions in one click.
The data is securely stored on-disk using [SQLcipher][], which uses 256-bit AES. The database is password-protected, but you can store the password in your OS keyring. Since the database is encrypted, you can put it in your dropbox (or similar account), to share it between computers.

Projectpad consists of two applications:

1. the GUI `projectpad` application, which allows you to enter/edit data, search it, open websites and so on;
2. the command-line `ppcli` application, which allows you to run commands, connect to servers, open files of interest and so on.

## GUI application

The application allows you to manage your database of projects info. It is organized in three panes:

- projects
- project items (servers, project notes, project point of interests, server links)
- project item contents (for servers that may be a number of sub-items)

At the top of the second pane we can see the project environments (development, staging, uat and prod).

Notes are especially interesting, you author them in markdown syntax.

And full-text search is supported.

See [the help](https://github.com/emmanueltouzery/projectpad2/wiki/Help) for more details.

## Command-line application

The command-line application loads all commands, servers, and files of interest, and displays them in a flat list, that you filter by typing and navigate using arrow keys. The application can execute commands, log you on ssh servers, edit configuration files, tail log files or fetch them, and so on.

[sqlcipher]: https://www.zetetic.net/sqlcipher/
