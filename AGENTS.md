# Rust Code Style Guidelines

### Functional-First Approach
- Prefer pure functions and immutable data transformations
- Use functional combinators: `map`, `filter`, `fold`, `find` where appropriate
- **Use loops where appropriate** - not everything needs to be functional
- Chain operations with iterator methods when it improves readability

### Declarative Style
- Express intent through function composition
- Use custom utility functions for common patterns
- Prefer `match` expressions over if-else chains
- Short, focused functions with clear names
- **Use destructuring**: `let MyType { field1, field2 } = something;` when appropriate

### Rust Idioms
- Heavy use of `impl Iterator` return types where it makes sense
- Custom traits for common operations (`MutateTrait`)
- Use `Iterator::find` or `find_map` instead of for loops where it improves clarity
- **Use `Iterator::any` or `Iterator::find` where appropriate** - prefer iterator methods over manual loops with early returns when checking for existence or finding elements
- **Use `find` when you want to find an element matching a predicate** - returns `Option<T>` where T is the element type
- **Use `find_map` when you want to find and transform an element** - returns `Option<U>` where U is different from the element type. Don't use `find_map` if you're just returning the same element unchanged - use `find` instead
- Avoid verbose procedural code - keep logic concise and declarative
- **Don't do this** - An example
//instead of this
if name.is_none() {
    for (_entity, transform, entity_name) in mob_query.iter() {
        if let Some(distance) = raycast_to_entity(ray_origin, ray_direction, transform.translation, 0.5, max_distance) {
            if distance < max_distance {
                name = Some(entity_name.to_string());
                break;
            }
        }
    }
}
//do this
name = name.or_else(|| {
    mob_query.iter().find_map(|(_, transform, entity_name)| {
        raycast_to_entity(ray_origin, ray_direction, transform.translation, 0.5, max_distance)
            .is_some_and(|d| d < max_distance)
            .then(|| entity_name.to_string())
    })
});
- **Use (multi line) let chains where appropriate** - prefer 
`if let Some(x) = foo
   && let Some(y) = bar`
and
`if something()
   && let Some(y) = bar`
and
`if let Some(x) = foo
   && something()`
    over nested `if let`/`if`s or sequences of `.and_then()`
Here's a let chain example:
`
fn process(opt_user: Option<User>, opt_config: Option<Config>) {
    if let Some(user) = opt_user
        && user.is_active
        && let Some(config) = opt_config
        && config.feature_enabled("premium")
    {
        println!("Active premium user: {}", user.name);
    } else {
        println!("Fallback path");
    }
}
`
- **Avoid early returns** - prefer using `if/else` expressions, conditional operators, and let chains instead of early returns for more functional/declarative code style. Don't use let else return. There are some algorithms that are best written with early returns though.
- **Domain modelling** - It's often a good idea to model parts of the domain by making named instances of types, either const or let locals, or in some cases an fn that returns an instance of some type. Also can be quite useful to make your own new types to describe various things, and then perhaps make some const instances of your types to describe various things in the domain, possibly as associated constants in an impl block.

### Bevy ECS Component Design Heuristic
- **If all entities with component A will have component B and all entities with component B will have component A, then merge components A and B into one struct component**
- **Components should usually be named by the behavior that they impart.** - If you're making a component for mobs that walk around randomly, name it something like `WalkAroundRandomly`. Don't name it `Sheep` or `SheepWalk` just because the mob is a sheep. Do name it `Sheep` if there's various different mob types and you want to distinguish the sheep mobs from the other ones by giving them their own component.

### Build Preferences
- **Development**: Prefer debug builds for fast iteration (`cargo run`, `cargo check`)
- **Testing**: Use release builds only when I ask for one
- Debug builds compile much faster, prioritize development speed over runtime speed during active development

## Documentation Links

### Bevy 0.17.2
- Docs: https://docs.rs/bevy/0.17.2/bevy/
- Book: https://bevyengine.org/learn/book/getting-started/
- Examples: https://github.com/bevyengine/bevy/tree/main/examples

### bevy_sprite3d
- Docs: https://docs.rs/bevy_sprite3d/
