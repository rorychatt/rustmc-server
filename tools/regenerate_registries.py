import os
import json

VANILLA_DATA_DIR = "/Users/rorychatt/git/rorychatt/rustmc-server/.cache/vanilla-data/data/minecraft/"
REGISTRIES_DIR = "/Users/rorychatt/git/rorychatt/rustmc-server/server/data/registries/v775/"
ORDER_FILE = "/Users/rorychatt/git/rorychatt/rustmc-server/server/tests/data/vanilla_registry_order.json"

REGISTRY_NAMES = [
    "cat_sound_variant",
    "cat_variant",
    "chicken_sound_variant",
    "chicken_variant",
    "cow_sound_variant",
    "cow_variant",
    "dimension_type",
    "enchantment",
    "frog_variant",
    "painting_variant",
    "pig_sound_variant",
    "pig_variant",
    "wolf_sound_variant",
    "wolf_variant",
    "zombie_nautilus_variant"
]

def map_enchantment_slots(data):
    # data is the dict loaded from enchantment json
    effects = data.get("effects", {})
    if "minecraft:attributes" in effects:
        attrs = effects["minecraft:attributes"]
        slots = data.get("slots", [])
        
        # Mapping logic
        if slots == ["mainhand", "offhand"]:
            mapped_slot = "hand"
        elif slots == ["any"]:
            mapped_slot = "any"
        elif len(slots) > 0:
            mapped_slot = slots[0]
        else:
            mapped_slot = "any"
            
        for attr in attrs:
            attr["slot"] = mapped_slot

def main():
    # Load order file
    if os.path.exists(ORDER_FILE):
        with open(ORDER_FILE, "r") as f:
            order_data = json.load(f)
    else:
        order_data = {}

    for reg_name in REGISTRY_NAMES:
        src_path = os.path.join(VANILLA_DATA_DIR, reg_name)
        if not os.path.isdir(src_path):
            print(f"Skipping {reg_name}: directory not found at {src_path}")
            continue
            
        # List and sort all json files alphabetically
        filenames = sorted([f for f in os.listdir(src_path) if f.endswith(".json")])
        
        registry_entries = []
        entry_ids = []
        
        for filename in filenames:
            entry_id = f"minecraft:{filename[:-5]}"
            filepath = os.path.join(src_path, filename)
            
            with open(filepath, "r") as f:
                data = json.load(f)
                
            if reg_name == "enchantment":
                map_enchantment_slots(data)
                
            registry_entries.append({
                "id": entry_id,
                "data": data
            })
            entry_ids.append(entry_id)
            
        # Write registry entries list
        dest_filepath = os.path.join(REGISTRIES_DIR, f"{reg_name}.json")
        with open(dest_filepath, "w") as f:
            json.dump(registry_entries, f, indent=2)
        print(f"Regenerated {dest_filepath} with {len(registry_entries)} entries.")
        
        # Update order data
        reg_key = f"minecraft:{reg_name}"
        order_data[reg_key] = entry_ids

    # Write updated order file
    with open(ORDER_FILE, "w") as f:
        json.dump(order_data, f, indent=2)
    print(f"Updated ordering snapshot: {ORDER_FILE}")

if __name__ == "__main__":
    main()
