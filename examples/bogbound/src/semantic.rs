use crate::model::{Element, Form};
use anny::{hnsw::Hnsw, metric::Cosine};
use std::collections::HashMap;

const DIM: usize = ese::DIMENSIONS;
type RecipeIndex = Hnsw<f32, Cosine, DIM, 16, 4, 32, 64, 12>;

#[derive(Debug, Clone)]
pub struct CompiledSpell {
    pub element: Element,
    pub form: Form,
    pub name: String,
    pub description: String,
    pub confidence: f32,
}

pub struct SpellCompiler {
    index: RecipeIndex,
    recipes: HashMap<u32, (Element, Form)>,
}

impl SpellCompiler {
    pub fn new() -> Self {
        let mut index = RecipeIndex::new(Cosine, 0xb06b0a7d);
        let mut recipes = HashMap::new();
        for element in Element::ALL {
            for form in Form::ALL {
                let text = prototype(element, form);
                let node = index.insert(ese::encode_single(text));
                recipes.insert(node, (element, form));
            }
        }
        Self { index, recipes }
    }

    pub fn compile(&self, element: Element, form: Form) -> CompiledSpell {
        let phrase = format!(
            "{} {} {}",
            element.label(),
            form.label(),
            prototype(element, form)
        );
        let vector = ese::encode_single(phrase);
        let (distance, node) = self
            .index
            .search(&vector)
            .into_iter()
            .next()
            .expect("spell catalog");
        let (compiled_element, compiled_form) = self.recipes[&node];
        CompiledSpell {
            element: compiled_element,
            form: compiled_form,
            name: format!("{} {}", compiled_element.label(), compiled_form.label()),
            description: description(compiled_element, compiled_form).into(),
            confidence: (1.0 - distance).clamp(0.0, 1.0),
        }
    }
}

fn prototype(element: Element, form: Form) -> &'static str {
    match (element, form) {
        (Element::Ember, Form::Bolt) => "ember fire burning bolt arrow projectile",
        (Element::Ember, Form::Orbit) => "ember fire burning orbit circling flame",
        (Element::Ember, Form::Swarm) => "ember fire burning swarm many homing sparks",
        (Element::Ember, Form::Nova) => "ember fire burning nova explosion pulse",
        (Element::Frost, Form::Bolt) => "frost ice frozen bolt arrow projectile",
        (Element::Frost, Form::Orbit) => "frost ice frozen orbit circling crystals",
        (Element::Frost, Form::Swarm) => "frost ice frozen swarm many homing snowflakes",
        (Element::Frost, Form::Nova) => "frost ice frozen nova blizzard pulse",
        (Element::Storm, Form::Bolt) => "storm lightning electric bolt arrow projectile",
        (Element::Storm, Form::Orbit) => "storm lightning electric orbit circling sparks",
        (Element::Storm, Form::Swarm) => "storm lightning electric swarm many homing wisps",
        (Element::Storm, Form::Nova) => "storm lightning electric nova thunder pulse",
        (Element::Thorn, Form::Bolt) => "thorn nature poison bolt arrow projectile",
        (Element::Thorn, Form::Orbit) => "thorn nature poison orbit circling leaves",
        (Element::Thorn, Form::Swarm) => "thorn nature poison swarm many homing seeds",
        (Element::Thorn, Form::Nova) => "thorn nature poison nova bramble pulse",
    }
}

fn description(element: Element, form: Form) -> &'static str {
    match (element, form) {
        (Element::Ember, Form::Bolt) => "A fast firebolt that burns its target.",
        (Element::Ember, Form::Orbit) => "Three flames circle you and ignite enemies.",
        (Element::Ember, Form::Swarm) => "Homing sparks hunt nearby enemies.",
        (Element::Ember, Form::Nova) => "A periodic ring of searing fire.",
        (Element::Frost, Form::Bolt) => "An ice shard that slows its target.",
        (Element::Frost, Form::Orbit) => "Three frozen crystals guard your path.",
        (Element::Frost, Form::Swarm) => "Snow wisps home in and slow enemies.",
        (Element::Frost, Form::Nova) => "A freezing pulse slows everything nearby.",
        (Element::Storm, Form::Bolt) => "A lightning bolt that chains to nearby enemies.",
        (Element::Storm, Form::Orbit) => "Orbiting sparks arc between close enemies.",
        (Element::Storm, Form::Swarm) => "A flock of lightning wisps seeks targets.",
        (Element::Storm, Form::Nova) => "A thunder pulse chains across the horde.",
        (Element::Thorn, Form::Bolt) => "A poisonous thorn that restores a little health.",
        (Element::Thorn, Form::Orbit) => "Circling leaves poison foes and heal you.",
        (Element::Thorn, Form::Swarm) => "Living seeds seek foes and siphon vitality.",
        (Element::Thorn, Form::Nova) => "A bramble pulse poisons foes and restores health.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_visible_pairs_compile_predictably() {
        let compiler = SpellCompiler::new();
        for element in Element::ALL {
            for form in Form::ALL {
                let spell = compiler.compile(element, form);
                assert_eq!((spell.element, spell.form), (element, form));
                assert!(spell.confidence > 0.5);
            }
        }
    }
}
