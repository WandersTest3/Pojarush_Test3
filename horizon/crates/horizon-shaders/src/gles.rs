//! Pojarush: adjustments that let a translated pack program run on OpenGL ES, whose GLSL lacks
//! a few qualifiers desktop GL and Vulkan have. Each one changes the picture as little as
//! possible: a `noperspective` varying becomes a perspective-correct one (identical for the
//! full-screen passes that use it, slightly different only on slanted geometry).
use naga::{Binding, Interpolation, Module};

fn fix(binding: &mut Option<Binding>) {
    if let Some(Binding::Location { interpolation, .. }) = binding
        && *interpolation == Some(Interpolation::Linear)
    {
        *interpolation = Some(Interpolation::Perspective);
    }
}

/// Rewrites `module` in place for OpenGL ES.
pub fn for_gles(module: &mut Module) {
    let handles: Vec<_> = module.types.iter().map(|(h, _)| h).collect();
    for h in handles {
        let ty = module.types[h].clone();
        if let naga::TypeInner::Struct { mut members, span } = ty.inner {
            let mut changed = false;
            for m in &mut members {
                let before = m.binding.clone();
                fix(&mut m.binding);
                changed |= before != m.binding;
            }
            if changed {
                module.types.replace(
                    h,
                    naga::Type { name: ty.name, inner: naga::TypeInner::Struct { members, span } },
                );
            }
        }
    }
    for ep in &mut module.entry_points {
        for arg in &mut ep.function.arguments {
            fix(&mut arg.binding);
        }
        if let Some(result) = &mut ep.function.result {
            fix(&mut result.binding);
        }
    }
}
