//! The copy of a part of one PHP value into another (SE-6.5 to SE-6.7).
//!
//! A copier keeps the identity of the objects and references that it copied, so the copies of two nodes that share an
//! object or a reference share its copy. It does not recurse: a container is made empty at once and filled from a list
//! of tasks, so the depth of a value does not grow the Rust stack (RQ-006).

use std::collections::HashMap;

use polyspec_php_serialization::{ArrayId, Node, ObjectId, PhpValue, ReferenceId};

use crate::error::{internal, Error};

enum Task {
    Array { from: ArrayId, to: ArrayId },
    Object { from: ObjectId, to: ObjectId },
    Reference { from: ReferenceId, to: ReferenceId },
}

/// Copies nodes of one value into others.
pub(crate) struct Copier<'a> {
    source: &'a PhpValue,
    objects: HashMap<ObjectId, ObjectId>,
    references: HashMap<ReferenceId, ReferenceId>,
    tasks: Vec<Task>,
}

impl<'a> Copier<'a> {
    pub(crate) fn new(source: &'a PhpValue) -> Self {
        Self { source, objects: HashMap::new(), references: HashMap::new(), tasks: Vec::new() }
    }

    /// Copies a node of the source into `target` with every node below it, and returns the node of the copy, which the
    /// caller places. An object or a reference that this copier copied before is the same copy.
    pub(crate) fn copy(&mut self, target: &mut PhpValue, node: &Node) -> Result<Node, Error> {
        let top = self.shallow(target, node)?;
        while let Some(task) = self.tasks.pop() {
            self.fill(target, task)?;
        }
        Ok(top)
    }

    fn shallow(&mut self, target: &mut PhpValue, node: &Node) -> Result<Node, Error> {
        Ok(match node {
            Node::Null => Node::Null,
            Node::Bool(value) => Node::Bool(*value),
            Node::Int(value) => Node::Int(*value),
            Node::Float(value) => Node::Float(*value),
            Node::String(bytes) => Node::String(bytes.clone()),
            Node::Array(from) => {
                let to = target.new_array();
                self.tasks.push(Task::Array { from: *from, to });
                Node::Array(to)
            }
            Node::Object(from) => {
                if let Some(to) = self.objects.get(from) {
                    return Ok(Node::Object(*to));
                }
                let to = target.new_object();
                self.objects.insert(*from, to);
                self.tasks.push(Task::Object { from: *from, to });
                Node::Object(to)
            }
            Node::Reference(from) => {
                if let Some(to) = self.references.get(from) {
                    return Ok(Node::Reference(*to));
                }
                let to = target.new_reference(Node::Null).map_err(internal("a new reference is refused"))?;
                self.references.insert(*from, to);
                self.tasks.push(Task::Reference { from: *from, to });
                Node::Reference(to)
            }
        })
    }

    fn fill(&mut self, target: &mut PhpValue, task: Task) -> Result<(), Error> {
        let source = self.source;
        match task {
            Task::Array { from, to } => {
                let array = source.array(from).map_err(internal("the source names no array"))?;
                for (key, node) in array.iter() {
                    let copy = self.shallow(target, node)?;
                    target.set_element(to, key.clone(), copy).map_err(internal("a copied element is refused"))?;
                }
            }
            Task::Object { from, to } => {
                let object = source.object(from).map_err(internal("the source names no object"))?;
                for (name, node) in object.iter() {
                    let copy = self.shallow(target, node)?;
                    target.set_property(to, name.to_vec(), copy).map_err(internal("a copied property is refused"))?;
                }
            }
            Task::Reference { from, to } => {
                let node = source.reference(from).map_err(internal("the source names no reference"))?;
                let copy = self.shallow(target, node)?;
                target.set_reference(to, copy).map_err(internal("a copied reference is refused"))?;
            }
        }
        Ok(())
    }
}

/// A value whose top value is a copy of a node of the source; a reference gives the value that it holds (SE-6.5).
pub(crate) fn value_of(source: &PhpValue, node: &Node) -> Result<PhpValue, Error> {
    let node = match node {
        Node::Reference(id) => source.reference(*id).map_err(internal("the source names no reference"))?,
        other => other,
    };
    let mut value = PhpValue::new();
    let top = Copier::new(source).copy(&mut value, node)?;
    value.set_root(top).map_err(internal("a copied top value is refused"))?;
    Ok(value)
}
