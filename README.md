# 🐘 pgfine

CLI tool to help with postgresql database schema updates, migrations and versioning.


# Install

## From [crates.io](https://crates.io/crates/pgfine)

```bash
cargo install pgfine
```

## From [repository](https://gitlab.com/mrsk/pgfine)

```bash
git clone https://gitlab.com/mrsk/pgfine
cargo install --path ./pgfine
```

# Create a new project

- Choose some version controlled directory.
- Create git-ignored `env-local-db-0.sh` (as an example) file like this:

```bash
# Connection strings: https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING

# credentials to connect to target db to perform updates and migrations
export PGFINE_CONNECTION_STRING="..."

# credentials for creating a new database  refereced in the above connection string (usually postgres db with user postgres).
export PGFINE_ADMIN_CONNECTION_STRING="..."

# path pointing to pgfine project, a good choice would be "./pgfine"
export PGFINE_DIR="./pgfine"
```

Environment variables need to be activated when using `pgfine`. You can do this by running:
```bash
source env-local-db-0.sh
```

To initialize `pgfine` project run:
```bash
pgfine init
```

This will create directory for storing all pgfine project data:

```
./pgfine/
├── create
│   ├── 00-create-role.sql
│   └── 01-create-database.sql
├── drop
│   ├── 00-drop-database.sql
│   └── 01-drop-role.sql
├── functions
├── migrations
├── roles
├── tables
├── constraints
└── views
```

Modify newly created `./pgfine/create/*.sql` and `./pgfine/drop/*.sql` scripts if needed.



# Create a database

- Modify `./pgfine/create/*` scripts if needed.
- Setup environment and run:

```bash
pgfine migrate
```

Two extra tables will be created:

- `pgfine_objects`: contains a list of managed pgfine objects and their hashes.
- `pgfine_migrations`: Contains a list of executed migrations. Selecting the max value should reveal the current state of database. The first migration will be inserted as empty string.


# Making changes to database

- Apply any changes to database schema objects in `./pgfine/**/*.sql`.
- All the chagnes related with tables should be implemented via `./pgfine/migrations/*` scripts.
- For all other objects (not tables) it is enough to modify a related create/alter script. (ex. `./pgfine/views/public.view0.sql`)
- Filenames for database objects must be of format `<schema>.<name>.sql`.
- Run
```bash
pgfine migrate
```

- Test your fresh db maybe.
- Commit all files to version control.




# Rollbacks

- Restore database object scripts from previous commits
- Create a new migration script if rollback involves data.
- Apply changes the same way: 
```bash
pgfine migrate
```


# Database objects

Database objects are:
- tables
- views
- indexes
- constraints
- functions
- ...

Each database object has coresponding create/alter script in pgfine project directory (see bellow for details). Filenames must consist of schema name and object name and `.sql` extension (example: `./pgfine/tables/public.some_table_0.sql`).

Database object scripts are executed when `pgfine` attempts to create or alter database object; except for tables - `pgfine` won't attempt to alter or drop tables, these changes have to be implemented using migration scripts.

Sometimes object needs to be dropped and created instead of updating it in place (one such case is when argument is removed from function definition). Drop script is generated using object id.


## Tables

Example `./pgfine/tables/public.table0.sql`:
```sql
create table table0 (
    id bigserial primary key
);

-- create indexes
-- create constraints
-- create rules
-- create triggers

```

Table constraints and indeces can be stored along with tables. But to modify them you will have to write migration scripts.

If you have circular foreign key dependencies you should define those constraints in a separate `./pgfine/constraints/` files to break the cycle.


## Views

Example `./pgfine/views/public.view0.sql`:
```sql
-- it is recommended to include "or replace", otherwise it will be dropped and created again each time changes are made.
create or replace view view0 as
select t0.id
from table0 t0
join table1 t1 on t1.id = t0.id

-- create indexes maybe
```

## Constraints

The schema part of constraint identifier should represent the schema of associated table. (Constraints do not dirrectly belong to particular schema of database, but they are associated with tables.)

When constraint is modified it will always be dropped and created again.

Example `./pgfine/constraints/public.table1_t0_id_fk.sql`:
```sql
alter table table1
add constraint table1_t0_id_fk foreign key (t0_id) references table1 (id);
```


Postgres allows you to have the same name constraints assigned to different tables. But pgfine will only work with uniquely defined constraints per schema.


# Commands

## `pgfine init`

- Initializes pgfine project at path `PGFINE_DIR`.


## `pgfine migrate`

### If database is missing:
Creates an up to date fresh databaes using `PGFINE_ADMIN_CONNECTION_STRING` and skips all migration scripts

### If database exists:

- Uses `PGFINE_CONNECTION_STRING` credentials to connect to a target database.
- Applies new scripts in `./pgfine/migrations/` and inserts executed scripts into `pgfine_migrations` table.
- Scans all objects in pgfine project dir and calculates update order to satisfy dependency tree.
- Attempts to update each object whose script hash does not match the one in the `pgfine_objects` table (or drop the object if it was deleted).
- Updates `pgfine_objects` table with newest information.


## `pgfine drop --no-joke`

- Uses `PGFINE_ADMIN_CONNECTION_STRING` credentials to connect to database.
- Uses executes `/pgfine/drop/*.sql` scripts to drop database and role.



# Assumptions

- Passwords, database names and roles can only have alphanumeric characters and underscore.
- Each script filename must uniquely identify correspoinding database object.
- Constraint names must be unique in a whole schema.
- Filename information is used to track dependencies between objects using simple whole word search, assuming default `public` schema.
- Each new file in `./pgfine/migrations/` is assumed to be increasing in alphabetical order.
- First we attempt to execute database_object script (which is usually `CREATE OR REPLACE`). If it fails we attempt to `DROP` (including dependencies if necesary) and `CREATE` a new version.
- empty string is the name of the first migration

# Timeouts

If your database is huge it will probably take some time to execute migrations if those are involved in moving the data. Timeouts should be disabled in `pgfine`. If you find that it hangs in the middle of transaction you might check what is holding the locks, or just kill the `pgfine` process manually and retry again.

## Configured on server side

    statement_timeout (integer)

        Abort any statement that takes more than the specified number of milliseconds, starting from the time the command arrives at the server from the client. If log_min_error_statement is set to ERROR or lower, the statement that timed out will also be logged. A value of zero (the default) turns this off.

        Setting statement_timeout in postgresql.conf is not recommended because it would affect all sessions.

    idle_in_transaction_session_timeout (integer)

        Terminate any session with an open transaction that has been idle for longer than the specified duration in milliseconds. This allows any locks held by that session to be released and the connection slot to be reused; it also allows tuples visible only to this transaction to be vacuumed. See Section 24.1 for more details about this.

        The default value of 0 disables this feature.

# Alternatives

At the current stage pgfine is not the best thing in the world. You might also want to check these alternatives:
- [refinery](https://github.com/rust-db/refinery)
- [flyway](https://flywaydb.org/)
- [diesel.rs](https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/)
- [dbmigrate](https://github.com/Keats/dbmigrate)
- [and more...](https://wiki.postgresql.org/wiki/Change_management_tools_and_techniques)


# Plan for 1.0.0

- [x] support for circular constraints (by adding `./pgfine/constraints`)
- [ ] more types of database objects (roles, triggers, rules?, indices.. ?)
- [ ] support tls
- [ ] example projects at `./example/`
- [ ] ability to override dependencies in comment section when standard resolution fails
- [x] implement `PGFINE_DIR`
- [ ] explain errors better in `database::migrate`, `database::drop`, `project::init`
- [x] document timeouts
- [ ] make README.md readable
- [x] build dependencies lazily? (does not allow to drop if error happens)
- [ ] drop missing objects with deps


# Post 1.0.0 plan

- [ ] operations in single transaction if possible
- [ ] configurable search schemas
- [ ] make execute order deterministic
- [ ] ignore comments in scripts when resolving dependencies
- [ ] support stable rust
- [ ] support for initial data (can be achieved by creating custom functions to initialize the data)
- [ ] generate project from existing database
