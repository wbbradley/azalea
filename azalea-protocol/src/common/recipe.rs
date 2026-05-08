use std::io::{self, Cursor, Write};

use azalea_buf::{AzBuf, AzBufVar, BufReadError};
use azalea_inventory::{DataComponentPatch, ItemStack, ItemStackData};
use azalea_registry::{
    HolderSet,
    builtin::{DataComponentKind, ItemKind},
    data::TrimPattern,
    identifier::Identifier,
};

/// [`azalea_registry::builtin::RecipeDisplay`]
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub enum RecipeDisplayData {
    Shapeless(ShapelessCraftingRecipeDisplay),
    Shaped(ShapedCraftingRecipeDisplay),
    Furnace(FurnaceRecipeDisplay),
    Stonecutter(StonecutterRecipeDisplay),
    Smithing(SmithingRecipeDisplay),
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct ShapelessCraftingRecipeDisplay {
    pub ingredients: Vec<SlotDisplayData>,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct ShapedCraftingRecipeDisplay {
    #[var]
    pub width: u32,
    #[var]
    pub height: u32,
    pub ingredients: Vec<SlotDisplayData>,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct FurnaceRecipeDisplay {
    pub ingredient: SlotDisplayData,
    pub fuel: SlotDisplayData,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
    #[var]
    pub duration: u32,
    pub experience: f32,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct StonecutterRecipeDisplay {
    pub input: SlotDisplayData,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct SmithingRecipeDisplay {
    pub template: SlotDisplayData,
    pub base: SlotDisplayData,
    pub addition: SlotDisplayData,
    pub result: SlotDisplayData,
    pub crafting_station: SlotDisplayData,
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct Ingredient {
    pub allowed: HolderSet<ItemKind, Identifier>,
}

/// [`azalea_registry::builtin::SlotDisplay`]
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub enum SlotDisplayData {
    Empty,
    AnyFuel,
    WithAnyPotion(Box<WithAnyPotionSlotDisplay>),
    OnlyWithComponent(Box<OnlyWithComponentSlotDisplay>),
    Item(ItemSlotDisplay),
    ItemStack(ItemStackSlotDisplay),
    Tag(TagSlotDisplay),
    Dyed(Box<DyedSlotDemo>),
    SmithingTrim(Box<SmithingTrimDemoSlotDisplay>),
    WithRemainder(Box<WithRemainderSlotDisplay>),
    Composite(CompositeSlotDisplay),
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct WithAnyPotionSlotDisplay {
    pub contents: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct OnlyWithComponentSlotDisplay {
    pub contents: SlotDisplayData,
    pub component: DataComponentKind,
}

#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct ItemSlotDisplay {
    pub item: ItemKind,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStackSlotDisplay {
    pub stack: ItemStack,
}

impl AzBuf for ItemStackSlotDisplay {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        // Mojang's `SlotDisplay$ItemStackSlotDisplay` codec is
        // `(item_kind_var, count_var, components)` — distinct from the
        // general inventory `ItemStack` codec which is
        // `(count_var, item_kind_var, components)` and treats
        // `count <= 0` as `Empty`. The empty case here is represented
        // by the sibling `SlotDisplayData::Empty` variant, so this
        // stack is always `Present` on the wire.
        let kind = ItemKind::azalea_read(buf)?;
        let count = i32::azalea_read_var(buf)?;
        let component_patch = DataComponentPatch::azalea_read(buf)?;
        Ok(Self {
            stack: ItemStack::Present(ItemStackData {
                kind,
                count,
                component_patch,
            }),
        })
    }

    fn azalea_write(&self, buf: &mut impl Write) -> io::Result<()> {
        match &self.stack {
            ItemStack::Present(d) => {
                d.kind.azalea_write(buf)?;
                d.count.azalea_write_var(buf)?;
                d.component_patch.azalea_write(buf)?;
            }
            ItemStack::Empty => {
                // Vanilla never sends an empty stack here — it uses
                // `SlotDisplayData::Empty`. Emit `Air|0` to keep the
                // codec total; this round-trips locally.
                ItemKind::Air.azalea_write(buf)?;
                0_i32.azalea_write_var(buf)?;
                DataComponentPatch::default().azalea_write(buf)?;
            }
        }
        Ok(())
    }
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct DyedSlotDemo {
    pub dye: SlotDisplayData,
    pub target: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct TagSlotDisplay {
    pub tag: Identifier,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct SmithingTrimDemoSlotDisplay {
    pub base: SlotDisplayData,
    pub material: SlotDisplayData,
    pub pattern: TrimPattern,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct WithRemainderSlotDisplay {
    pub input: SlotDisplayData,
    pub remainder: SlotDisplayData,
}
#[derive(AzBuf, Clone, Debug, PartialEq)]
pub struct CompositeSlotDisplay {
    pub contents: Vec<SlotDisplayData>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use azalea_buf::AzBuf;
    use azalea_registry::Registry;

    use super::*;

    /// Captured `ClientboundRecipeBookAdd` payload (sans packet-id
    /// prefix) from a vanilla MC 26.1 server for the shapeless recipe
    /// `dark_oak_log → 4 dark_oak_planks` (recipe id 350). Used to pin
    /// the SlotDisplay::ItemStack wire shape.
    const PLANKS_PAYLOAD: &[u8] = &[
        0x01, 0xde, 0x02, 0x00, 0x01, 0x06, 0x17, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f',
        b't', b':', b'd', b'a', b'r', b'k', b'_', b'o', b'a', b'k', b'_', b'l', b'o', b'g', b's',
        0x05, 0x2a, 0x04, 0x00, 0x00, 0x04, 0xcd, 0x02, 0x08, 0x00, 0x01, 0x01, 0x00, 0x17, b'm',
        b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'd', b'a', b'r', b'k', b'_', b'o',
        b'a', b'k', b'_', b'l', b'o', b'g', b's', 0x03, 0x00,
    ];

    /// Decode a captured `RecipeBookAdd` payload, walk to the lone
    /// entry's result slot, and assert it's the expected
    /// `(ItemKind, count)`.
    #[test]
    fn slot_display_item_stack_decodes_in_kind_then_count_order() {
        use crate::packets::game::c_recipe_book_add::ClientboundRecipeBookAdd;

        let mut cur = Cursor::new(PLANKS_PAYLOAD);
        let decoded = ClientboundRecipeBookAdd::azalea_read(&mut cur).expect("decode");
        assert_eq!(decoded.entries.len(), 1);
        let entry = &decoded.entries[0];
        assert_eq!(entry.contents.id, 350, "recipe id");
        let RecipeDisplayData::Shapeless(s) = &entry.contents.display else {
            panic!("expected Shapeless");
        };
        let SlotDisplayData::ItemStack(disp) = &s.result else {
            panic!("expected ItemStack result, got {:?}", s.result);
        };
        let ItemStack::Present(d) = &disp.stack else {
            panic!("expected Present");
        };
        assert_eq!(d.kind, ItemKind::DarkOakPlanks, "kind from item-id slot");
        assert_eq!(d.count, 4, "count from count slot");
    }

    /// Round-trip: encode an `ItemStackSlotDisplay` and decode it back;
    /// the result must compare equal. Guards against codec drift
    /// between read and write.
    #[test]
    fn slot_display_item_stack_round_trip() {
        let original = ItemStackSlotDisplay {
            stack: ItemStack::Present(ItemStackData::new(ItemKind::DarkOakPlanks, 4)),
        };
        let mut buf = Vec::new();
        original.azalea_write(&mut buf).expect("write");
        let mut cur = Cursor::new(buf.as_slice());
        let round_tripped = ItemStackSlotDisplay::azalea_read(&mut cur).expect("read");
        assert_eq!(original, round_tripped);
        assert_eq!(cur.position() as usize, buf.len(), "cursor consumed all bytes");
    }

    /// Direct sanity-check: the wire bytes in
    /// `azalea_write(ItemStackSlotDisplay)` are NOT the same as
    /// `azalea_write(ItemStack)` — the leading varints are swapped.
    /// Pins the bug being fixed.
    #[test]
    fn slot_display_item_stack_differs_from_inventory_codec() {
        let kind = ItemKind::DarkOakPlanks;
        let count = 4_i32;
        let stack = ItemStack::Present(ItemStackData::new(kind, count));

        let mut display_bytes = Vec::new();
        ItemStackSlotDisplay { stack: stack.clone() }
            .azalea_write(&mut display_bytes)
            .unwrap();

        let mut inv_bytes = Vec::new();
        stack.azalea_write(&mut inv_bytes).unwrap();

        // Inventory codec writes `(count_var, kind_var, ...)`.
        // Display codec writes `(kind_var, count_var, ...)`.
        // For non-pathological values these byte streams differ.
        assert_ne!(
            display_bytes, inv_bytes,
            "SlotDisplay and inventory codecs must encode differently",
        );

        // Spot-check the leading byte: display starts with the kind
        // varint, inventory starts with the count varint. DarkOakPlanks
        // and count=4 are both small (<128), so each is a single-byte
        // varint with no continuation bit set.
        assert!(kind.to_u32() < 128, "test fixture assumes small registry index");
        assert!(count < 128, "test fixture assumes small count");
        assert_eq!(
            display_bytes[0], kind.to_u32() as u8,
            "display codec leads with kind varint",
        );
        assert_eq!(
            inv_bytes[0], count as u8,
            "inventory codec leads with count varint",
        );
    }
}
