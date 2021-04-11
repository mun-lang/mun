## Hot Reloading Structs

To understand how we might use hot reloading of structures, let's create the skeleton for a simulation.
Start by creating a new project called `buoyancy`:

```bash
mun new buoyancy
```

and replace the contents of `src/mod.mun` with Listing 3-13.

The `new_sim` function constructs a `SimContext`, which maintains the simulation's state, and the `sim_update` function will be called every frame to update the state of `SimContext`. 
As Mun doesn't natively support logging, we'll use the extern function `log_f32` to log values of the `f32` type.

The subject of our simulation will be buoyancy; i.e. the upward force exerted by a fluid on a (partially) immersed object that allows it to float. 
Currently, all our simulation does it to log the elapsed time, every frame.

Filename: src/mod.mun

```mun,no_run
{{#include ../listings/ch03-structs/listing13.mun}}
```

<span class="caption">
Listing 3-13: The buoyancy simulation with state stored in `SimContext`
</span>

To be able to run our simulation, we need to embed it in a host language.
Listing 3-14 illustrates how to do this in Rust.

```rust,no_run,noplaypen
{{#include ../listings/ch03-structs/listing14.rs}}
```

<span class="caption">
Listing 3-14: The buoyancy simulation embedded in Rust
</span>

Now that we have a runnable host program, let's fire it up and see that hot reloading magic at work! 
First we need to start the build watcher:

```bash
mun build --watch --manifest-path=buoyancy/mun.toml
```

This will create the initial `mod.munlib` that we can use to run our host program in Rust:

```bash
cargo run -- buoyancy/target/mod.munlib
```

Your console should now receive a steady steam of 0.04... lines, indicating that the simulation is indeed running at 25 Hz. 
Time to add some logic.

### Insert Struct Fields

Our simulation will contain a spherical object with radius *r* and density *d<sub>o</sub>* that is dropped from an initial height *h* into a body of water with density *d<sub>w</sub>*. 
The simulation also takes the gravity, *g*, into account, but for the sake of simplicity we'll only consider vertical movement.
Let's add this to the `SimContext` struct and update the `new_sim` function accordingly, as shown in Listing 3-15.

```mun
# pub fn main() {
#   new_sim();
#   new_sphere();
# }
{{#include ../listings/ch03-structs/listing15.mun:3:41}}
```

<span class="caption">
Listing 3-15: Struct definitions of the buoyancy simulation
</span>

#### Runtime Struct Field Initialization

Upon successful compilation, the runtime will hot reload the new structs. 
Memory of newly added structs will recursively be zero initialized. 
This means that all fundamental types of a newly added structs and its child structs will be equal to zero.

We can verify this by replacing the `log_f32(elapsed_secs)` statement with:

```mun,ignore
{{#include ../listings/ch03-structs/listing15.mun:44}}
```

Indeed the console now receives a stream of `0` lines. 
Luckily there is a trick that we can employ to still manually initialize our memory to desired values by using this behavior to our advantage. 
Let's first add `token: u32` to the `SimContext`:

```mun,ignore
{{#include ../listings/ch03-structs/listing16.mun:7}}
```

and set it to zero in the `new_sim` function:

```mun,ignore
{{#include ../listings/ch03-structs/listing16.mun:26}}
```

As before, the `token` value will be initialized to zero when the library has been hot reloaded. 
Next, we add a `hot_reload_token` function that returns a non-zero `u32` value, e.g. `1`:

```mun,ignore
{{#include ../listings/ch03-structs/listing16.mun:45:47}}

```

Finally, we add this `if` statement to the `sim_update` function:

```mun,ignore
{{#include ../listings/ch03-structs/listing16.mun:50:56}}
```

This piece of code will be triggered every time the `hot_reload_token` function returns a different value, but only once - allowing us to initialize the value of `SimContext`.

### Edit Struct Fields

Time to add the actual logic for simulating buoyancy. 
The formula for calculating the buoyancy force is *force = submerged volume \* water density \* gravity*.

```mun,ignore
{{#include ../listings/ch03-structs/listing17.mun:51:73}}
```

Next we need to convert force into acceleration using *acc = force / mass*. 
We don't readily have the sphere's mass available, but we can derive it using the sphere's volume and density: *mass = volume \* density*. 
Instead of doing this every frame, let's replace the sphere's `density` field with a `mass` field:

```mun,no_run
{{#include ../listings/ch03-structs/listing17.mun:10:15}}
```

and pre-calculate it on construction:

```mun,ignore
{{#include ../listings/ch03-structs/listing17.mun:30:43}}
```

To initialize the sphere's `mass` field, we can employ the same trick as before; this time only initializing the sphere and incrementing `hot_reload_token` to `2`:

```mun,ignore
{{#include ../listings/ch03-structs/listing17.mun:80:84}}
```

Editing a field's name is only one of three ways that you can edit struct fields in Mun. 
In order of priority, these are the changes that the Mun Runtime is able to detect:

1) If an old field and new field have the same name and type, they must have remained unchanged. 
   In this case, the field can be **moved**.
2) If an old field and new field have the same name, they must be the same field. 
   In this case, we accept a **type conversion** and the field can potentially be **moved**.
3) If an old field and new field have different names but the same type, the field *could* have been renamed. 
   As there can be multiple candidates with the same type, we accept the **renamed** and potentially **moved** field that is closest to the original index of the old field.

Some restrictions do apply:

* A struct cannot simultaneously be **renamed** and its fields **edited**.
* A struct field cannot simultaneously be **renamed** and undergo a **type conversion**.

In both of the above cases, the difference will be recognized as two separate changes: an insertion and a deletion of the struct/field.

### Remove Struct Fields

We now have all of the building blocks necessary to finish our buoyancy simulation.
If the sphere is (partially) submerged, we calculate and add the buoyancy acceleration to the velocity.
We also always subtract the gravitational acceleration from the velocity to ensure that the sphere drops into the water.

> One important thing to take into account when running simulations is to multiply the accelerations and velocities with the elapsed time, as we are working in discrete time.

Last but not least, let's log the sphere's height to the console, so we can verify that the simulation is running correctly.

```mun,ignore
{{#include ../listings/ch03-structs/listing17.mun:86:105}}
```

When the simulation has been hot reloaded, the console should now log height values of the ball that are indicative of a sphere bobbing on the waves.

Now that our simulation is completed, we no longer need the `token` field, `hot_reload_token` function, and `if` statement.
The `token` field can be safely removed and the simulation hot reloaded without losing any state.

Well done!
You've just had your first experience of hot reloading structs.
