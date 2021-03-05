# 🐘 pgfine

CLI tool to help with postgresql database schema updates, migrations and versioning.

The goal of pgfine is to provide project structure declarative as much as possible:
- all database objects have their corresponding create script.
- migration scripts should only be needed to update data-full objects - tables.


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
# All variables are mandatory to avoid mixed environments.
# Connection strings: https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNSTRING
# no need to provide sslmode parameter.

# credentials to connect to target db to perform updates and migrations.
# role and database will be created if missing (using admin connection).
export PGFINE_CONNECTION_STRING="..."

# credentials for creating a new database  refereced in the above connection string (usually postgres db with user postgres).
export PGFINE_ADMIN_CONNECTION_STRING="..."

# path pointing to pgfine project, a good choice would be "./pgfine"
export PGFINE_DIR="./pgfine"

# role prefix to make them unique per database.
# if your plan is to have a single db per postgresql instance you can set it to "" and forget it.
# role names should be referenced like "{pgfine_role_prefix}role_name" in all the scripts.
# if you plan to use global roles you should create them manualy or in ./pgfine/create/ scripts
export PGFINE_ROLE_PREFIX="prod_"

# path to root certificate. No tls mode will be attempted if this is set to an empty string.
# https://www.postgresql.org/docs/current/ssl-tcp.html
export PGFINE_ROOT_CERT=""
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
├── schemas
│   └── public.sql
├── constraints
├── triggers
├── policies
├── extensions
├── types
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
    ```sql
        create table if not exists pgfine_objects (
            po_id text primary key,
            po_md5 text,
            po_script text,
            po_path text,
            po_depends_on text[],
            po_required_by text[]
        );
    ```
- `pgfine_migrations`: contains a list of executed migrations. Selecting the max value should reveal the current state of database. The first migration will be inserted as empty string.
    ```sql
        create table if not exists pgfine_migrations (
            pm_id text primary key
        );
    ```


# Making changes to database

- Apply any changes to database objects in `./pgfine/**/*.sql`.
- Table chagnes should be additionally implemented via `./pgfine/migrations/*` scripts.
- Run
```bash
pgfine migrate
```

- Test your fresh db maybe.
- Commit all files to version control.



The table summarizes the way you should deliver changes (auto means it is enough to modify corresponding object script):

Object type | Create | Drop      | Alter
----------- | ------ | --------- | ----
Table       | auto   | migration | migration
View        | auto   | auto      | auto
Function    | auto   | auto      | auto
Constraint  | auto   | auto      | auto
Trigger     | auto   | auto      | auto
Policy      | auto   | auto      | auto
Schema      | auto   | auto      | migration
Role        | auto   | auto      | auto
Extension   | auto   | auto      | migration
Type        | auto   | auto      | migration
Function    | auto   | auto      | auto


During the update there is short time period when the policies are dropped (if needed). This might be a security issue. This should be fixed once updates are 
applied in single transaction (needs more investigation).


# Migration scripts

Table changes can not be applied just by dropping the table and creating a new one without loosing the data. 
Therefore these changes must be delivered using migration scripts. Table scripts in the pgfine project
must represent the latest version of the object after applying all migration scripts.

Scripts are located at `./pgfine/migrations/`. These scripts are executed in alphabetical order before 
updating all the other database objects.

If your migration depends on other database objects (a new table column associated with a function maybe)
it is recommended to create those objects (if not exists) in the migration sctipt. This is to avoid problems with 
old versions of databases were mentioned objects don't yet exist. In the future schema verification process will 
be developed to show which migration scripts are broken.



# Rollbacks

- Restore database object scripts from previous commits
- Create a new migration script if it involves changing table.
- Apply changes the same way: 
```bash
pgfine migrate
```


# Database objects

Database objects are:
- tables
- views
- triggers
- constraints
- policies
- functions
- roles
- schemas
- extensions
- types

Filenames for database objects must be of specific format :
- tables: `./pgfine/tables/<schema>.<name>.sql`
- views: `./pgfine/views/<schema>.<name>.sql`
- functions: : `./pgfine/functions/<schema>.<name>.sql`
- triggers: `./pgfine/triggers/<schema>.<table>.<name>.sql`
- constraints: `./pgfine/constraints/<schema>.<table>.<name>.sql`
- policies: `./pgfine/policies/<schema>.<table>.<name>.sql`
- roles: `./pgfine/roles/<name>.sql`
- schemas: `./pgfine/schemas/<name>.sql`
- extensions: `./pgfine/extensions/<name>.sql`
- types: `./pgfine/types/<schema>.<name>.sql`


Each file contains script to create that object.

Updates are done by dropping the object and creating a new one.

Drop scripts are generated by object type and object name. Tables will never be dropped automatically - they have to be dropper/updated using migration scripts or manually.


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

## Functions

During update all overidden functions will be dropped (if modified) and created again.


Some default permissions might be applied on a function when dropping and creating it.
In the function script you might want to add extra statement to alter default privileges:
```sql
revoke execute on function some_function from public;
```


## Constraints

Example `./pgfine/constraints/public.table1.t0_id_fk.sql`:
```sql
alter table table1
add constraint t0_id_fk foreign key (t0_id) references table1 (id);
```

## Policies

Example `./pgfine/policies/public.table1.policy1.sql`:
```sql
create policy policy1
on public.table1;
```

Policy script should not target specific role. Role assignments should be done in role scripts by altering given policy.


## Roles

Example `./pgfine/roles/role0.sql`:
```sql
create role {pgfine_role_prefix}role0;
grant usage on schema schema0 to {pgfine_role_prefix}role0;
```

All permissions assignments should be done in role scripts.
Role objects will always be dropped and newly created when executing `pgfine migrate`.
This is to avoid default permissions assignment when other objects are recreated.

# Commands

## `pgfine init`

- Initializes pgfine project at path `PGFINE_DIR`.


## `pgfine migrate`

### If database is missing:

- Executes `./pgfine/create/` scripts to create role and database (using admin connection).
- Creates pgfine tables.
- Creates all database objects defined in pgfine project.

### If database exists:

- Attempts to drop all dirty objects by comparing `pgfine_objects` table and project contents.
- Attempts to create all missing objects.

## `pgfine drop --no-joke`

- Force drops all roles found in pgfine_objects table.
- Force drops all roles found in project.
- Executes `./pgfine/drop/` scripts to drop role and database (using admin connection).


# Assumptions

- Passwords, database names and roles can only have alphanumeric characters and underscores.
- Filename information is used to track dependencies between objects using simple whole word search, assuming default `public` schema.
- Triggers, constraints and policies are assumed to not be required by other objects (always safe to drop).
- Each new file in `./pgfine/migrations/` is assumed to be increasing in alphabetical order.
- empty string is the name of the first migration (inserted if no migrations exist)
- `{pgfine_role_prefix}` text should not be used for other porpuses as for database-role prefix in your scripts.
- no md5 comparison is done for schemas, types and extensions objects, changes should be done using migration scripts. (will attempt to drop them if script is deleted)
- default drop database scrip assumes postgres v13, if you use lower version you should add script to drop connections.


# Alternatives

At the current stage pgfine is not the best thing in the world. You might also want to check these alternatives:
- [refinery](https://github.com/rust-db/refinery)
- [flyway](https://flywaydb.org/)
- [diesel.rs](https://docs.rs/diesel_migrations/1.4.0/diesel_migrations/)
- [dbmigrate](https://github.com/Keats/dbmigrate)
- [and more...](https://wiki.postgresql.org/wiki/Change_management_tools_and_techniques)


# Breaking changes

## 1 -> 2

- object type is now part of object id.

Migration steps
- make your database up to date. (by running `pgfine migrate`)
- update `pgfine`
- drop tables `pgfine_objects` and `pgfine_migrations`
- run `pgfine migrate`


# Post 2.0.0 plan

- [ ] validate if object is self referenced
- [ ] validate table schema when hash has changed (by creating separate DB? and comparing?) before applying all other updates
- [ ] `PGFINE_ALLOW_DROP` variable to protect production envs
- [ ] example projects at `./example/`
- [ ] documentation https://documentation.divio.com/ https://jacobian.org/series/great-documentation/
- [ ] `./pgfine/initial/` execute after the database is created 
- [ ] `./pgfine/final/` execute after the database objects are created
- [ ] operations in single transaction if possible
- [ ] configurable search schemas
- [x] make execute order deterministic
- [ ] ignore comments in scripts when resolving dependencies
- [ ] support stable rust
- [ ] generate project from existing database
- [ ] solution for for functions required by tables?
- [ ] user defined drop scripts
- [x] attempt do to drop without deps
- [x] drop all fucntions having the same name



