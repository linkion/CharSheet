"""JSON Schemas (draft 2020-12) for every file in data/srd.

build.py writes them to data/srd/schema/, and check.py validates the data against them
with a small validator of its own, so no third-party package is needed.
"""

from common import (
    ABILITIES, CONDITIONS, CREATURE_TYPES, DAMAGE_TYPES, SCHOOLS, SIZES, SKILLS,
)

DRAFT = "https://json-schema.org/draft/2020-12/schema"


def obj(required, optional=None, extra=False):
    props = dict(required)
    props.update(optional or {})
    return {"type": "object", "properties": props, "required": list(required), "additionalProperties": extra}


def arr(items, **kw):
    return {"type": "array", "items": items, **kw}


def enum(values):
    return {"enum": list(values)}


def nullable(schema):
    out = dict(schema)
    t = out.get("type")
    if t is None:
        return {"anyOf": [schema, {"type": "null"}]}
    out["type"] = [t, "null"] if isinstance(t, str) else [*t, "null"]
    if "enum" in out:
        out["enum"] = [*out["enum"], None]
    return out


STR = {"type": "string"}
INT = {"type": "integer"}
NUM = {"type": "number"}
BOOL = {"type": "boolean"}
MD = {"type": "string", "description": "Markdown"}
DICE = {"type": "string", "pattern": r"^\d+d\d+([+-]\d+)?$"}


def ref(category):
    return {"type": "string", "pattern": rf"^{category}:[a-z0-9-]+$"}


def base(category, extra_required=None, extra_optional=None):
    """Fields every record has; `category` is the id prefix and category value."""
    req = {
        "id": ref(category),
        "name": STR,
        "category": {"const": category},
        "page": {"type": "integer", "minimum": 5, "maximum": 364},
    }
    req.update(extra_required or {})
    req["summary"] = STR
    req["markdown"] = MD
    return obj(req, extra_optional)


COST = nullable(obj({"amount": NUM, "unit": enum(["cp", "sp", "ep", "gp", "pp"])}))
WEIGHT = nullable(NUM)
NAMED_MD = obj({"name": STR, "markdown": MD})

ACTION = obj(
    {"name": STR, "markdown": MD},
    {
        "usage": STR,
        "attack": obj(
            {"kind": enum(["melee", "ranged", "melee_or_ranged"]), "bonus": INT},
            {"reach": INT, "range": INT, "long_range": INT},
        ),
        "save": obj({"ability": enum(ABILITIES), "dc": INT}),
        "damage": arr(obj({"average": INT, "dice": nullable(DICE), "type": enum(DAMAGE_TYPES)})),
    },
)

SCHEMAS = {
    "spells": base("spell", {
        "level": {"type": "integer", "minimum": 0, "maximum": 9},
        "school": enum(SCHOOLS),
        "classes": arr(ref("class"), minItems=1),
        "casting_time": obj(
            {"text": STR, "activation": enum(["action", "bonus_action", "reaction", "time"])},
            {"amount": INT, "unit": enum(["minute", "hour"]), "trigger": STR},
        ),
        "ritual": BOOL,
        "range": obj(
            {"kind": enum(["self", "touch", "feet", "miles", "sight", "unlimited", "special"]), "text": STR},
            {"value": INT, "area": obj({"shape": STR, "size": INT})},
        ),
        "components": obj(
            {"v": BOOL, "s": BOOL, "m": BOOL, "text": STR},
            {"material": STR, "cost_gp": INT, "consumed": BOOL},
        ),
        "duration": obj(
            {"concentration": BOOL, "text": STR, "kind": enum(["instantaneous", "timed", "until_dispelled", "special"])},
            {"amount": INT, "unit": enum(["round", "minute", "hour", "day"]), "up_to": BOOL, "or_triggered": BOOL},
        ),
        "concentration": BOOL,
    }, {
        "higher_levels": MD,
        "cantrip_upgrade": MD,
        "saves": arr(enum(ABILITIES)),
        "attack": enum(["melee", "ranged"]),
        "damage": arr(obj({"dice": DICE, "type": enum(DAMAGE_TYPES)})),
        "damage_types": arr(enum(DAMAGE_TYPES)),
    }),
    "monsters": base("monster", {
        "group": enum(["monster", "animal"]),
        "size": arr(enum(SIZES), minItems=1),
        "creature_type": enum(CREATURE_TYPES),
        "tags": arr(STR),
        "alignment": STR,
        "ac": INT,
        "initiative": obj({"mod": INT, "score": INT}),
        "hp": obj({"average": INT, "formula": nullable(DICE)}),
        "speed": obj({}, {
            "walk": INT, "fly": INT, "swim": INT, "climb": INT, "burrow": INT, "hover": BOOL, "notes": arr(STR),
        }),
        "abilities": obj({
            a[:3]: obj({"score": {"type": "integer", "minimum": 1, "maximum": 30}, "mod": INT, "save": INT})
            for a in ABILITIES
        }),
        "senses": obj({"passive_perception": INT}, {
            "blindsight": INT, "darkvision": INT, "tremorsense": INT, "truesight": INT,
            "blindsight_note": STR, "darkvision_note": STR, "tremorsense_note": STR, "truesight_note": STR,
            "notes": arr(STR),
        }),
        "languages": STR,
        "cr": obj({"value": NUM, "text": STR, "xp": INT, "pb": INT}, {"xp_lair": INT}),
    }, {
        "family": STR,
        "swarm": obj({"size": enum(SIZES)}),
        "skills": {"type": "object", "propertyNames": enum(SKILLS), "additionalProperties": INT},
        "resistances": obj({"damage": arr(enum(DAMAGE_TYPES))}, {"notes": arr(STR)}),
        "vulnerabilities": obj({"damage": arr(enum(DAMAGE_TYPES))}, {"notes": arr(STR)}),
        "immunities": obj({"damage": arr(enum(DAMAGE_TYPES)), "conditions": arr(enum(CONDITIONS))}, {"notes": arr(STR)}),
        "gear": arr(STR),
        "traits": arr(ACTION),
        "actions": arr(ACTION),
        "bonus_actions": arr(ACTION),
        "reactions": arr(ACTION),
        "legendary_actions": obj({
            "uses": nullable(INT), "uses_lair": nullable(INT), "intro": MD, "actions": arr(ACTION),
        }),
    }),
    "magic_items": base("magic_item", {
        "aliases": arr(STR),
        "attunement": obj({"required": BOOL}, {"by": STR, "classes": arr(ref("class")), "spellcaster": BOOL}),
        "item_type": enum(["wondrous_item", "weapon", "armor", "potion", "ring", "wand", "staff", "rod", "scroll"]),
        "rarity": enum(["common", "uncommon", "rare", "very_rare", "legendary", "artifact", "varies"]),
        "cursed": BOOL,
    }, {
        "subtype": STR,
        "rarity_by_variant": arr(obj({"variant": STR, "rarity": enum(["common", "uncommon", "rare", "very_rare", "legendary", "artifact"])})),
        "charges": obj({"max": INT}, {"regain": {"type": ["string", "integer"]}, "regain_when": STR}),
    }),
    "classes": base("class", {
        "primary_ability": arr(enum(ABILITIES), minItems=1),
        "primary_ability_text": STR,
        "hit_die": enum([6, 8, 10, 12]),
        "saving_throws": arr(enum(ABILITIES), minItems=2, maxItems=2),
        "skill_choices": obj({"count": INT, "from": arr(enum(SKILLS), minItems=1)}),
        "weapon_proficiencies": STR,
        "armor_training": STR,
        "starting_equipment": MD,
        "level_1": MD,
        "multiclass": MD,
        "features_table": obj({
            "columns": arr(STR),
            "rows": arr({
                "type": "object",
                "required": ["level", "proficiency_bonus", "features"],
                "properties": {
                    "level": {"type": "integer", "minimum": 1, "maximum": 20},
                    "proficiency_bonus": {"type": "integer", "minimum": 2, "maximum": 6},
                    "features": arr(STR),
                    "spell_slots": arr(INT),
                },
            }, minItems=20, maxItems=20),
        }),
        "features": arr(obj({"level": INT, "name": STR, "markdown": MD}), minItems=1),
        "option_groups": arr(obj({
            "name": STR, "intro": MD,
            "options": arr(obj({"name": STR, "repeatable": BOOL, "markdown": MD}, {"prerequisite": STR})),
        })),
        "spell_list": arr(ref("spell")),
        "subclasses": arr(ref("subclass"), minItems=1),
    }, {"tool_proficiencies": STR}),
    "subclasses": base("subclass", {
        "class": ref("class"),
        "features": arr(obj({"level": INT, "name": STR, "markdown": MD}), minItems=1),
    }, {
        "spells": arr(obj({"level": INT, "spells": arr(ref("spell"))}, {"group": STR})),
    }),
    "species": base("species", {
        "creature_type": enum(CREATURE_TYPES),
        "size": arr(enum(SIZES), minItems=1),
        "size_text": STR,
        "speed": INT,
        "traits": arr(NAMED_MD, minItems=1),
    }),
    "backgrounds": base("background", {
        "ability_scores": arr(enum(ABILITIES), minItems=3, maxItems=3),
        "feat": obj({"name": STR, "id": ref("feat")}),
        "skill_proficiencies": arr(enum(SKILLS), minItems=2, maxItems=2),
        "tool_proficiency": STR,
        "equipment": MD,
    }),
    "feats": base("feat", {
        "feat_category": enum(["origin", "general", "fighting_style", "epic_boon"]),
        "repeatable": BOOL,
        "benefits": arr(NAMED_MD),
    }, {"prerequisite": STR, "prerequisite_level": INT}),
    "weapons": base("weapon", {
        "weapon_category": enum(["simple", "martial"]),
        "ranged": BOOL,
        "damage": obj({"dice": {"type": "string", "pattern": r"^(\d+d\d+|\d+)$"}, "type": enum(DAMAGE_TYPES)}),
        "properties": arr(obj(
            {"id": ref("weapon_property"), "name": STR},
            {"range": obj({"normal": INT, "long": INT}), "ammunition": STR, "damage": DICE, "note": STR},
        )),
        "mastery": ref("mastery_property"),
        "weight_lb": WEIGHT,
        "cost": COST,
    }),
    "weapon_properties": base("weapon_property"),
    "mastery_properties": base("mastery_property"),
    "armor": base("armor", {
        "armor_category": enum(["light", "medium", "heavy", "shield"]),
        "ac": obj({"text": STR}, {"base": INT, "dex_cap": nullable(INT), "bonus": INT}),
        "strength_required": nullable(INT),
        "stealth_disadvantage": BOOL,
        "don_doff": STR,
        "weight_lb": WEIGHT,
        "cost": COST,
    }),
    "tools": base("tool", {
        "tool_category": enum(["artisans_tools", "other"]),
        "ability": enum(ABILITIES),
        "utilize": arr(STR, minItems=1),
        "craft": arr(STR),
        "weight_lb": WEIGHT,
        "cost": COST,
    }, {"variants": arr(STR)}),
    "gear": base("gear", {"weight_lb": WEIGHT, "cost": COST}, {
        "cost_text": STR,
        "variants": arr({"type": "object", "required": ["name"], "properties": {"name": STR, "weight_lb": WEIGHT, "cost": COST}}),
    }),
    "mounts_vehicles": {
        "anyOf": [
            base("mount", {"vehicle_type": {"const": "mount"}, "carrying_capacity_lb": WEIGHT, "cost": COST}),
            base("vehicle", {"vehicle_type": {"const": "tack_or_drawn"}, "weight_lb": WEIGHT, "cost": COST}),
            base("vehicle", {
                "vehicle_type": {"const": "ship"}, "speed_mph": NUM, "crew": nullable(INT), "passengers": nullable(INT),
                "cargo_tons": nullable(NUM), "ac": nullable(INT), "hp": nullable(INT), "damage_threshold": nullable(INT),
                "cost": COST,
            }),
        ],
    },
    "services": base("service", {"service_type": STR, "cost": COST}, {"per": STR, "availability": STR}),
    "glossary": base("glossary", {
        "tag": nullable(enum(["action", "area_of_effect", "attitude", "condition", "hazard"])),
        "see_also": arr(ref("glossary")),
        "see_also_text": arr(STR),
    }),
    "conditions": base("condition", {
        "glossary": ref("glossary"),
        "effects": arr(NAMED_MD, minItems=1),
    }, {"max_level": INT, "per_level": obj({"d20_tests": INT, "speed_ft": INT})}),
    "rules": base("rule", {
        "page_end": INT,
        "path": arr(STR, minItems=1),
        "parent": nullable({"type": "string", "pattern": r"^rule:[a-z0-9/-]+$"}),
        "children": arr({"type": "string", "pattern": r"^rule:[a-z0-9/-]+$"}),
    }),
}
# Rule ids are paths: rule:playing-the-game/d20-tests/ability-checks.
SCHEMAS["rules"]["properties"]["id"] = {"type": "string", "pattern": r"^rule:[a-z0-9/-]+$"}

INDEX = obj({
    "id": {"type": "string", "pattern": r"^[a-z_]+:[a-z0-9/-]+$"},
    "name": STR,
    "category": STR,
    "aliases": arr(STR),
    "keywords": arr(STR),
    "page": INT,
    "file": {"type": "string", "pattern": r"^[a-z_]+\.json$"},
    "summary": STR,
})


def documents():
    """{file name: schema document} for every data file plus the index."""
    out = {}
    for name, item in SCHEMAS.items():
        out[f"{name}.schema.json"] = {"$schema": DRAFT, "title": f"SRD 5.2.1 {name}", "type": "array", "items": item}
    out["index.schema.json"] = {"$schema": DRAFT, "title": "SRD 5.2.1 search index", "type": "array", "items": INDEX}
    return out


def validate(value, schema, path="$"):
    """Yields an error string for every place value breaks schema (the subset used above)."""
    if "anyOf" in schema:
        if not any(not list(validate(value, s, path)) for s in schema["anyOf"]):
            yield f"{path}: matches no allowed shape"
        return
    if "const" in schema and value != schema["const"]:
        yield f"{path}: {value!r} is not {schema['const']!r}"
    if "enum" in schema and value not in schema["enum"]:
        yield f"{path}: {value!r} not one of {schema['enum'][:8]}{'…' if len(schema['enum']) > 8 else ''}"
        return
    t = schema.get("type")
    if t:
        types = [t] if isinstance(t, str) else t
        ok = {
            "string": isinstance(value, str),
            "integer": isinstance(value, int) and not isinstance(value, bool),
            "number": isinstance(value, (int, float)) and not isinstance(value, bool),
            "boolean": isinstance(value, bool),
            "object": isinstance(value, dict),
            "array": isinstance(value, list),
            "null": value is None,
        }
        if not any(ok[x] for x in types):
            yield f"{path}: {type(value).__name__} is not {t}"
            return
    if value is None:
        return
    if isinstance(value, str) and "pattern" in schema:
        import re

        if not re.search(schema["pattern"], value):
            yield f"{path}: {value!r} doesn't match {schema['pattern']}"
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            yield f"{path}: {value} < {schema['minimum']}"
        if "maximum" in schema and value > schema["maximum"]:
            yield f"{path}: {value} > {schema['maximum']}"
    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            yield f"{path}: fewer than {schema['minItems']} items"
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            yield f"{path}: more than {schema['maxItems']} items"
        if "items" in schema:
            for i, v in enumerate(value):
                yield from validate(v, schema["items"], f"{path}[{i}]")
    if isinstance(value, dict):
        props = schema.get("properties", {})
        for k in schema.get("required", []):
            if k not in value:
                yield f"{path}: missing {k}"
        for k, v in value.items():
            if k in props:
                yield from validate(v, props[k], f"{path}.{k}")
            elif "propertyNames" in schema:
                yield from validate(k, schema["propertyNames"], f"{path} key")
                if isinstance(schema.get("additionalProperties"), dict):
                    yield from validate(v, schema["additionalProperties"], f"{path}.{k}")
            elif schema.get("additionalProperties") is False:
                yield f"{path}: unexpected field {k}"
