

use std::collections::HashSet;
use std::collections::HashMap;
use std::fs;
use std::path;
use std::iter::FromIterator;
use std::str::FromStr;
use path::PathBuf;
use anyhow;
use anyhow::Context;
use md5::Md5;
use md5::Digest;
use hex;
use postgres;
use crate::utils;


#[cfg(test)]
mod tests;

fn get_project_path() -> anyhow::Result<PathBuf> {
    let project_path_str = utils::read_env_var("WO_DIR")
        .context("get_project_path error: failed to read env variable WO_DIR")?;
    return Ok(PathBuf::from(project_path_str));
}

fn get_role_prefix() -> anyhow::Result<String> {
    let role_prefix = utils::read_env_var("WO_ROLE_PREFIX")
        .context("get_role_prefix error: failed to read env variable WO_ROLE_PREFIX")?;
    return Ok(role_prefix);
}

fn get_create_script_00() -> (String, String) {
    let filename = String::from("00-create-role.sql");
    let content = "
-- available parameters for substitution {param}:
-- -- database_name
-- -- role_name
-- -- password
-- parameters are taken from WO_CONNECTION_STRING env variable
-- parameters are validated to contain only alphanum cahracters and underscores
CREATE ROLE \"{role_name}\"
WITH
LOGIN PASSWORD '{password}'
SUPERUSER;
".into();
    return (filename, content);
}

fn get_create_script_01() -> (String, String) {
    let filename = String::from("01-create-database.sql");
    let content = "
-- available parameters for substitution {param}:
-- -- database_name
-- -- role_name
-- -- password
-- parameters are taken from WO_CONNECTION_STRING env variable
-- parameters are validated to contain only alphanum cahracters and underscores
CREATE DATABASE \"{database_name}\"
WITH
OWNER = \"{role_name}\"
TEMPLATE = template0
ENCODING = 'UTF8'
LC_COLLATE = 'en_US.UTF-8'
LC_CTYPE = 'en_US.UTF-8'
TABLESPACE = pg_default
CONNECTION LIMIT = 10;
".into();
    return (filename, content);
}

fn get_drop_script_00() -> (String, String) {
    let filename = String::from("00-drop-database.sql");
    let content = "
-- available parameters for substitution {param}:
-- -- database_name
-- -- role_name
-- -- password
-- parameters are taken from WO_CONNECTION_STRING env variable
-- parameters are validated to contain only alphanum cahracters and underscores
DROP DATABASE IF EXISTS \"{database_name}\" WITH (FORCE);
".into();
    return (filename, content);
}

fn get_drop_script_01() -> (String, String) {
    let filename = String::from("01-drop-role.sql");
    let content = "
-- available parameters for substitution {param}:
-- -- database_name
-- -- role_name
-- -- password
-- parameters are taken from WO_CONNECTION_STRING env variable
-- parameters are validated to contain only alphanum cahracters and underscores
DROP ROLE IF EXISTS \"{role_name}\";
".into();
    return (filename, content);
}

fn get_default_schema_script() -> (String, String) {
    let filename = String::from("public.sql");
    let content = "create schema public;".into();
    return (filename, content);
}

pub fn init() -> anyhow::Result<()> {
    
    let project_path = get_project_path()
        .context("init error: failed to get project path")?;
    
    if project_path.exists() {
        println!("project directory already exists at {:?}", project_path);
        return Ok(());
    }

    fs::create_dir_all(&project_path)?;
    fs::create_dir(project_path.join("create"))?;
    fs::create_dir(project_path.join("drop"))?;
    fs::create_dir(project_path.join("tables"))?;
    fs::create_dir(project_path.join("views"))?;
    fs::create_dir(project_path.join("functions"))?;
    fs::create_dir(project_path.join("roles"))?;
    fs::create_dir(project_path.join("migrations"))?;
    fs::create_dir(project_path.join("constraints"))?;
    fs::create_dir(project_path.join("triggers"))?;
    fs::create_dir(project_path.join("schemas"))?;
    fs::create_dir(project_path.join("policies"))?;
    fs::create_dir(project_path.join("extensions"))?;
    fs::create_dir(project_path.join("types"))?;


    {
        let (filename, content) = get_create_script_00();
        let path_buf = project_path.join("create").join(filename);
        utils::write_file(&path_buf, &content)?;
    }

    {
        let (filename, content) = get_create_script_01();
        let path_buf = project_path.join("create").join(filename);
        utils::write_file(&path_buf, &content)?;
    }

    {
        let (filename, content) = get_drop_script_00();
        let path_buf = project_path.join("drop").join(filename);
        utils::write_file(&path_buf, &content)?;
    }

    {
        let (filename, content) = get_drop_script_01();
        let path_buf = project_path.join("drop").join(filename);
        utils::write_file(&path_buf, &content)?;
    }

    {
        let (filename, content) = get_default_schema_script();
        let path_buf = project_path.join("schemas").join(filename);
        utils::write_file(&path_buf, &content)?;
    }

    
    return Ok(());
}

fn validate_object_id(id: &str) -> anyhow::Result<()> {
    let object_type = get_object_type(id)
        .context(format!("could not parse type from object id {:?}", id))?;
    
    match object_type {
        DatabaseObjectType::Constraint => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 4 {
                bail!("constraint filename format shoud be <schema>.<table>.<name> {:?}", id);
            }
        },
        DatabaseObjectType::Trigger => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 4 {
                bail!("trigger filename format shoud be <schema>.<table>.<name> {:?}", id);
            }
        },
        DatabaseObjectType::Policy => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 4 {
                bail!("policy filename format shoud be <schema>.<table>.<name> {:?}", id);
            }
        },
        DatabaseObjectType::Table => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 3 {
                bail!("table filename format shoud be <schema>.<name> {:?}", id);
            }
        },
        DatabaseObjectType::View => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 3 {
                bail!("view filename format shoud be <schema>.<name> {:?}", id);
            }
        },
        DatabaseObjectType::Function => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 3 {
                bail!("function filename format shoud be <schema>.<name> {:?}", id);
            }
        },
        DatabaseObjectType::Role => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 2 {
                bail!("role filename should not contain dots {:?}", id);
            }
        },
        DatabaseObjectType::Schema => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 2 {
                bail!("schema filename should not contain dots {:?}", id);
            }
        },
        DatabaseObjectType::Extension => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 2 {
                bail!("extension filename should not contain dots {:?}", id);
            }
        },
        DatabaseObjectType::Type => {
            let id_parts: Vec<&str> = id.split('.').collect();
            if id_parts.len() != 3 {
                bail!("type filename format shoud be <schema>.<name> {:?}", id);
            }
        },
    }
    return Ok(());
}

fn object_id_from_path(
    path_buf: &PathBuf,
    object_type: &DatabaseObjectType
) -> anyhow::Result<String> {
    
    let filestem = path_buf.file_stem()
        .ok_or(anyhow!("object_id_from_path error: could not parse filename {:?}", path_buf))?;
    let filestem_str = filestem.to_str()
        .ok_or(anyhow!("object_id_from_path error: could not parse filename {:?}", path_buf))?;

    let object_id;
    if *object_type == DatabaseObjectType::Role {
        let role_prefix = get_role_prefix()?;
        if filestem_str.contains(".") {
            bail!("filename for role objects should not be separated by dot, role prefix is specified via env variable");
        }
        object_id = format!("{}{}", role_prefix, filestem_str).to_lowercase();
    } else {
        object_id = filestem_str.to_lowercase();
    }

    let object_type_str = String::from(object_type);
    let object_id = format!("{}.{}", object_type_str, object_id);
    
    validate_object_id(&object_id)?;
    return Ok(object_id);
}

fn migration_id_from_path(path_buf: &PathBuf) -> anyhow::Result<String> {
    let filename = path_buf.file_name()
        .ok_or(anyhow!("migration_id_from_path error: could not parse filename {:?}", path_buf))?;
    let filename_str = filename.to_str()
        .ok_or(anyhow!("migration_id_from_path error: could not parse filename {:?}", path_buf))?;

    return Ok(filename_str.into());
}

fn prepare_script(
    script: &str,
    role_prefix: &str
) -> String {
    let result = script.replace("{wo_role_prefix}", role_prefix);
    return result;
}

fn load_objects_info_by_type(
    result: &mut HashMap<String, (DatabaseObjectType, PathBuf, String)>, 
    path_buf: &PathBuf,
    object_type: &DatabaseObjectType
) -> anyhow::Result<()> {
    let role_prefix = get_role_prefix()?;
    let ls_paths = utils::list_files(&path_buf)
        .context(format!("load_objects_info error: failed to list files at {:?}", path_buf))?;
    for ls_path in ls_paths {
        let object_id = object_id_from_path(&ls_path, &object_type)
            .context(format!("load_objects_info error: failed to parse object_id {:?}", ls_path))?;
        let script = utils::read_file(&ls_path)
            .context(format!("load_objects_info error: failed to read file {:?}", ls_path))?;
        
        let script = prepare_script(&script, &role_prefix);
        result.insert(object_id, (object_type.clone(), ls_path, script));
    }
    return Ok(());
}


fn load_objects_info(project_path: &PathBuf) -> anyhow::Result<HashMap<String, (DatabaseObjectType, PathBuf, String)>> {
    let mut result = HashMap::new();

    let path_buf = project_path.join("tables");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Table)?;

    let path_buf = project_path.join("views");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::View)?;

    let path_buf = project_path.join("functions");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Function)?;

    let path_buf = project_path.join("constraints");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Constraint)?;

    let path_buf = project_path.join("roles");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Role)?;

    let path_buf = project_path.join("triggers");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Trigger)?;

    let path_buf = project_path.join("schemas");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Schema)?;

    let path_buf = project_path.join("policies");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Policy)?;

    let path_buf = project_path.join("extensions");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Extension)?;

    let path_buf = project_path.join("types");
    load_objects_info_by_type(&mut result, &path_buf, &DatabaseObjectType::Type)?;

    return Ok(result);
}

fn get_search_term(
    object_id: &str,
    object_type: &DatabaseObjectType,
    search_schemas: &HashSet<String>
) -> anyhow::Result<Option<String>> {
    match object_type {
        DatabaseObjectType::Function |
        DatabaseObjectType::Table |
        DatabaseObjectType::Type |
        DatabaseObjectType::View => {
            let schema = get_schema(object_id)?;
            let name = get_name(object_id)?;
            if search_schemas.contains(schema) {
                return Ok(Some(name.clone().into()));
            } else {
                return Ok(Some(format!("{}.{}", schema, name)));
            }
        },
        DatabaseObjectType::Policy |
        DatabaseObjectType::Constraint |
        DatabaseObjectType::Trigger => return Ok(None),
        DatabaseObjectType::Extension |
        DatabaseObjectType::Role => {
            let name = get_name(object_id)?;
            return Ok(Some(name.clone().into()));
        }
        DatabaseObjectType::Schema => bail!("schema dependencies should be derived from object ids"),
    };
}

fn calc_required_by_for_schema(
    object_id: &str,
    objects_info: &HashMap<String, (DatabaseObjectType, PathBuf, String)>,
) -> anyhow::Result<HashSet<String>> {
    let mut result = HashSet::new();
    let schema = get_name(object_id)?;
    for (required_by_object_id, (object_type, _, script)) in objects_info {

        if *object_type == DatabaseObjectType::Extension
        || *object_type == DatabaseObjectType::Schema 
        || *object_type == DatabaseObjectType::Role
        {
            let contains = utils::contains_whole_word_ci(&script, &schema);
            if contains {
                result.insert(required_by_object_id.clone());
            }
            continue;
        }

        let required_by_schema = get_schema(required_by_object_id)?;
        if schema == required_by_schema {
            result.insert(required_by_object_id.clone());
        }
    }

    return Ok(result);
}

/*  role objects should only depend on other role objects */
fn calc_required_by_for_role(
    object_id: &str,
    objects_info: &HashMap<String, (DatabaseObjectType, PathBuf, String)>,
) -> anyhow::Result<HashSet<String>> {
    let mut result = HashSet::new();
    let role_name = get_name(object_id)?;
    for (required_by_object_id, (object_type, _, script)) in objects_info {

        if *object_type != DatabaseObjectType::Role {
            continue;
        }
        
        let contains = utils::contains_whole_word_ci(&script, &role_name);
        if contains {
            result.insert(required_by_object_id.clone());
        }
    }

    return Ok(result);
}

fn calc_required_by_for_object(
    object_id: &str,
    objects_info: &HashMap<String, (DatabaseObjectType, PathBuf, String)>,
    search_schemas: &HashSet<String>
) -> anyhow::Result<HashSet<String>> {
    let object_type = objects_info[object_id].0;
    
    if object_type == DatabaseObjectType::Schema {
        return calc_required_by_for_schema(object_id, objects_info);
    } else if object_type == DatabaseObjectType::Role {
        return calc_required_by_for_role(object_id, objects_info);
    }
    
    let mut result = HashSet::new();
    let search_term_opt = get_search_term(
        object_id,
        &object_type,
        search_schemas
    )?;

    if search_term_opt.is_none() {
        return Ok(result);
    }

    let search_term = search_term_opt.unwrap();
    for (required_by_object_id, (_, _, script)) in objects_info {
        if object_id == required_by_object_id {
            continue;
        }
        let contains = utils::contains_whole_word_ci(&script, &search_term);
        if contains {
            result.insert(required_by_object_id.clone());
        }
    }
    return Ok(result);
}

fn calc_required_by(
    objects_info: &HashMap<String, (DatabaseObjectType, PathBuf, String)>,
    search_schemas: &HashSet<String>
) -> anyhow::Result<HashMap<String, HashSet<String>>> {
    let mut result = HashMap::new();
    for (object_id, _) in objects_info {
        let required_by = calc_required_by_for_object(object_id, objects_info, &search_schemas)?;
        result.insert(object_id.clone(), required_by);
    }
    return Ok(result);
}

fn calc_depends_on_for_object(object_id: &str, required_by: &HashMap<String, HashSet<String>>) -> HashSet<String> {
    let mut result = HashSet::new();
    for (depends_on_object_id, required_by) in required_by {
        if object_id == depends_on_object_id {
            continue;
        }
        if required_by.contains(object_id) {
            result.insert(depends_on_object_id.clone());
        }
    }
    return result;
}

fn calc_depends_on(required_by: &HashMap<String, HashSet<String>>) -> HashMap<String, HashSet<String>>{
    let mut result = HashMap::new();
    for (object_id, _object_required_by) in required_by {
        let depends_on = calc_depends_on_for_object(object_id, required_by);
        result.insert(object_id.clone(), depends_on);
    }
    return result;
}

fn build_database_objects(
    mut objects_info: HashMap<String, (DatabaseObjectType, PathBuf, String)>,
    mut required_by: HashMap<String, HashSet<String>>,
    mut depends_on: HashMap<String, HashSet<String>>
) -> anyhow::Result<HashMap<String, DatabaseObject>> {
    let mut result = HashMap::new();
    let mut hasher = Md5::new();
    for (object_id, (_, path_buf, script)) in objects_info.drain() {
        let object_depends_on = depends_on.remove(&object_id).expect("depends_on.remove(&object_id)");
        let object_required_by = required_by.remove(&object_id).expect("required_by.remove(&object_id)");
        let id = object_id.clone();
        hasher.update(&script);
        let hash = hasher.finalize_reset();
        let hash_str = hex::encode(hash);
        let o = DatabaseObject {
            id,
            path_buf: path_buf,
            script: script,
            md5: hash_str,
            depends_on: object_depends_on,
            required_by: object_required_by,
        };
        result.insert(object_id, o);
    }
    return Ok(result);
}

fn resolve_dependencies(
    object_id: &String, 
    objects: &HashMap<String, DatabaseObject>,
    dependencies_vec: &mut Vec<String>,
    dependencies_set: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> anyhow::Result<()> {
    if dependencies_set.contains(object_id) {
        return Ok(());
    }

    if visited.contains(object_id) {
        bail!("resolve_dependencies error: cycle detected {:?}", object_id);
    }
    visited.insert(object_id.clone());

    let object = objects.get(object_id).unwrap();
    let mut new_dependencies_sorted: Vec<&String> = Vec::from_iter(&object.depends_on);
    new_dependencies_sorted.sort();
    
    for dep in new_dependencies_sorted {
        resolve_dependencies(
            &dep,
            &objects,
            dependencies_vec,
            dependencies_set,
            visited
        )?;
    }

    dependencies_set.insert(object_id.clone());
    dependencies_vec.push(object_id.clone());
    return Ok(());
}

fn calc_create_order(objects: &HashMap<String, DatabaseObject>) -> anyhow::Result<Vec<String>> {
    let mut dependencies_vec: Vec<String> = vec![];
    let mut dependencies_set: HashSet<String> = HashSet::new();

    let mut objects_sorted: Vec<&String> = Vec::new();
    let mut roles_sorted: Vec<&String> = Vec::new();

    for object_id in objects.keys() {
        let object_type = get_object_type(object_id)?;
        if object_type == DatabaseObjectType::Role {
            roles_sorted.push(object_id);
        } else {
            objects_sorted.push(object_id);
        }
    }
    objects_sorted.sort();
    roles_sorted.sort();
    objects_sorted.extend(roles_sorted);

    for object_id in objects_sorted {
        let mut visited: HashSet<String> = HashSet::new();
        resolve_dependencies(
            object_id, 
            objects, 
            &mut dependencies_vec, 
            &mut dependencies_set,
            &mut visited
        )?;
    }

    return Ok(dependencies_vec);
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum DatabaseObjectType {
    Table,
    View,
    Function,
    Constraint,
    Role,
    Trigger,
    Schema,
    Policy,
    Extension,
    Type,
}

impl From<&DatabaseObjectType> for String {
    fn from(t: &DatabaseObjectType) -> Self {
        match t {
            DatabaseObjectType::Table => "table".into(),
            DatabaseObjectType::View => "view".into(),
            DatabaseObjectType::Function => "function".into(),
            DatabaseObjectType::Constraint => "constraint".into(),
            DatabaseObjectType::Role => "role".into(),
            DatabaseObjectType::Trigger => "trigger".into(),
            DatabaseObjectType::Schema => "schema".into(),
            DatabaseObjectType::Policy => "policy".into(),
            DatabaseObjectType::Extension => "extension".into(),
            DatabaseObjectType::Type => "type".into(),
        }
    }
}

impl FromStr for DatabaseObjectType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let object_type = match s {
            "table" => DatabaseObjectType::Table,
            "view" => DatabaseObjectType::View,
            "function" => DatabaseObjectType::Function,
            "constraint" => DatabaseObjectType::Constraint,
            "role" => DatabaseObjectType::Role,
            "trigger" => DatabaseObjectType::Trigger,
            "schema" => DatabaseObjectType::Schema,
            "policy" => DatabaseObjectType::Policy,
            "extension" => DatabaseObjectType::Extension,
            "type" => DatabaseObjectType::Type,
            _ => bail!("could not convert object type from {:?}", s),
        };
        return Ok(object_type);
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseObject {
    pub id: String,
    pub path_buf: PathBuf,
    pub script: String,
    pub md5: String,
    pub depends_on: HashSet<String>,
    pub required_by: HashSet<String>,
}

fn get_id_part<'t>(
    id: &'t str, 
    i: usize
) -> anyhow::Result<&'t str> {
    validate_object_id(id)?;
    let id_parts: Vec<&str> = id.split('.').collect();
    return Ok(id_parts[i]);
}

pub fn get_object_type(id: &str) -> anyhow::Result<DatabaseObjectType> {
    let id_parts: Vec<&str> = id.split('.').collect();
    let object_type_str = id_parts[0];
    let object_type = DatabaseObjectType::from_str(object_type_str)
        .context(format!("could not parse object type from id {:?}", id))?;
    return Ok(object_type);
}

pub fn get_schema(id: &str) -> anyhow::Result<&str> {
    let object_type = get_object_type(id)?;

    match object_type {
        DatabaseObjectType::Constraint |
        DatabaseObjectType::Trigger |
        DatabaseObjectType::Policy |
        DatabaseObjectType::Table |
        DatabaseObjectType::View |
        DatabaseObjectType::Function |
        DatabaseObjectType::Type => get_id_part(id, 1),
        DatabaseObjectType::Role => bail!("role object id is not associated with schema {:?}", id),
        DatabaseObjectType::Schema => bail!("schema object id is not associated with another schema {:?}", id),
        DatabaseObjectType::Extension => bail!("extension object id is not associated with schema {:?}", id),
    }
}

pub fn get_table(id: &str) -> anyhow::Result<&str> {
    let object_type = get_object_type(id)?;

    match object_type {
        DatabaseObjectType::Constraint |
        DatabaseObjectType::Trigger |
        DatabaseObjectType::Policy => get_id_part(id, 2),
        DatabaseObjectType::Table => bail!("table object id is not associated with another table {:?}", id),
        DatabaseObjectType::View => bail!("view object id is not associated with table {:?}", id),
        DatabaseObjectType::Function => bail!("function object id is not associated with table {:?}", id),
        DatabaseObjectType::Role => bail!("role object id is not associated with table {:?}", id),
        DatabaseObjectType::Schema => bail!("schema object id is not associated with table {:?}", id),
        DatabaseObjectType::Extension => bail!("extension object id is not associated with table {:?}", id),
        DatabaseObjectType::Type => bail!("type object id is not associated with table {:?}", id),
    }
}

pub fn get_name(id: &str) -> anyhow::Result<&str> {
    let object_type = get_object_type(id)?;

    match object_type {
        DatabaseObjectType::Constraint |
        DatabaseObjectType::Trigger |
        DatabaseObjectType::Policy => get_id_part(id, 3),
        DatabaseObjectType::Table |
        DatabaseObjectType::View |
        DatabaseObjectType::Type |
        DatabaseObjectType::Function => get_id_part(id, 2),
        DatabaseObjectType::Role |
        DatabaseObjectType::Schema |
        DatabaseObjectType::Extension => get_id_part(id, 1),
    }
}


impl DatabaseObject {


    fn id_part(&self, i: usize) -> anyhow::Result<&str> {
        return get_id_part(&self.id, i);
    }

    pub fn object_type(&self) -> anyhow::Result<DatabaseObjectType> {
        let type_str = self.id_part(0)
            .context(format!("could not extract type from id {:?}", self.id))?;
        let result = DatabaseObjectType::from_str(type_str)
            .context(format!("could not type type from id {:?}", self.id))?;
        return Ok(result);
    }

    pub fn schema(&self) -> anyhow::Result<&str> {
        return get_schema(&self.id);
    }

    pub fn table(&self) -> anyhow::Result<&str> {
        return get_table(&self.id);
    }

    pub fn name(&self) -> anyhow::Result<&str> {
        return get_name(&self.id);
    }

    pub fn from_db_row(row: &postgres::Row) -> anyhow::Result<Self> {
        // po_id text primary key,
        // po_type text,
        // po_md5 text,
        // po_script text,
        // po_path text,
        // po_depends_on text[],
        // po_required_by text[]

        let po_id: String = row.try_get("po_id")?;
        let po_md5: String = row.try_get("po_md5")?;
        let po_script: String = row.try_get("po_script")?;
        let po_path: String = row.try_get("po_path")?;
        let po_depends_on: Vec<String> = row.try_get("po_depends_on")?;
        let po_required_by: Vec<String> = row.try_get("po_required_by")?;

        let path_buf = PathBuf::from(po_path);
        let depends_on = HashSet::from_iter(po_depends_on);
        let required_by = HashSet::from_iter(po_required_by);

        validate_object_id(&po_id)?;

        let result = DatabaseObject {
            id: po_id,
            path_buf,
            script: po_script,
            md5: po_md5,
            depends_on,
            required_by,
        };

        return Ok(result);
    }
}


pub struct DatabaseProject {
    pub project_dirpath: PathBuf,
    pub create_scripts: Vec<(PathBuf, String)>,
    pub drop_scripts: Vec<(PathBuf, String)>,
    pub migration_scripts: Vec<(String, String)>,
    pub objects: HashMap<String, DatabaseObject>,
}

impl DatabaseProject {
    fn from_path(project_path: &PathBuf) -> anyhow::Result<DatabaseProject> {

        let path_buf = project_path.join("create");
        let create_script_paths = utils::list_files(&path_buf)?;
        let mut create_scripts = vec![];
        for p in create_script_paths {
            let script = utils::read_file(&p)?;
            create_scripts.push((p, script));
        }

        let path_buf = project_path.join("drop");
        let drop_script_paths = utils::list_files(&path_buf)?;
        let mut drop_scripts = vec![];
        for p in drop_script_paths {
            let script = utils::read_file(&p)?;
            drop_scripts.push((p, script));
        }

        let path_buf = project_path.join("migrations");
        let migration_script_paths = utils::list_files(&path_buf)?;
        let mut migration_scripts = vec![];
        for p in migration_script_paths {
            let script = utils::read_file(&p)?;
            let migration_id = migration_id_from_path(&p)?;
            migration_scripts.push((migration_id, script));
        }
        
        // FIXME configurable search schemas???
        let mut search_schemas: HashSet<String> = HashSet::new();
        search_schemas.insert("public".into());

        let objects_info = load_objects_info(&project_path)?;
        let required_by = calc_required_by(&objects_info, &search_schemas)?;
        let depends_on = calc_depends_on(&required_by);
        let objects = build_database_objects(objects_info, required_by, depends_on)?;

        return Ok(DatabaseProject {
            project_dirpath: path_buf,
            create_scripts,
            drop_scripts,
            migration_scripts,
            objects,
        });
    }

    pub fn get_next_migration(&self, migration_id: &str) -> Option<(String, String)> {
        let migration_id = String::from(migration_id);
        for (next_migration_id, next_migration_script) in self.migration_scripts.iter() {
            if *next_migration_id > migration_id {
                return Some((next_migration_id.clone(), next_migration_script.clone()));
            }
        }
        return None;
    }

    pub fn get_create_order(&self) -> anyhow::Result<Vec<String>> {
        return calc_create_order(&self.objects);
    }
}


pub fn load() -> anyhow::Result<DatabaseProject> {
    let project_path = get_project_path()?;
    let database_project = DatabaseProject::from_path(&project_path)?;
    return Ok(database_project);
}

