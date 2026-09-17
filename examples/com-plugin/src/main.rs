//! Exercises identical typed construction in dynamic and monolithic hosts.

use neocogi_com::{Runtime, guid_from_qualified_name};
use neocogi_idl_example_contracts::{ExampleFactoryAbi, Extent};

/// Registers the selected plugin, acquires its checked factory, and mutates one object.
fn main() {
    let runtime = Runtime::global();
    register_plugin(&runtime);
    // The runtime verifies the complete transitive contract before bootstrap.
    let factory = runtime
        .activate_plugin_factory::<ExampleFactoryAbi>(guid_from_qualified_name("nc.examples.factory_plugin_component"))
        .expect("compatible example factory");
    let mut object = factory.create(Extent { width: 20, height: 10 }).expect("typed object construction");
    object.resize(Extent { width: 44, height: 31 }).expect("exclusive object mutation");
    let extent = object.extent().expect("inherited interface call");
    assert_eq!(extent, Extent { width: 44, height: 31 });
    println!("factory object: {} x {}", extent.width, extent.height);
}

/// Borrows immutable plugin metadata from the rlib retained by this executable.
#[cfg(feature = "monolith")]
fn register_plugin(runtime: &Runtime) {
    // SAFETY: Both the descriptor and its code are linked into this process.
    unsafe { runtime.register_factory_plugin_description(neocogi_example_com_plugin::factory_plugin_description()) }.expect("static plugin registration");
}

/// Loads the trusted sample plugin supplied as the executable's first argument.
#[cfg(not(feature = "monolith"))]
fn register_plugin(runtime: &Runtime) {
    let path = std::env::args_os().nth(1).expect("usage: com-plugin-host PATH_TO_PLUGIN");
    // SAFETY: This demonstration accepts its own trusted build artifact. The
    // runtime retains the mapping while any factory or object can reference it.
    unsafe { runtime.load_plugin_library(std::path::Path::new(&path)) };
}
