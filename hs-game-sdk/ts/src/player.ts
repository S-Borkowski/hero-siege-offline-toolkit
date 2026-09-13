export enum EquipmentSlot {
  Helm = 0,
  Chestplate = 1,
  Boots = 2,
  Gloves = 3,
  Belt = 4,
  Amulet = 5,
  Ring1 = 6,
  Ring2 = 7,
  MainHand = 8,
  OffHand = 9,
  Relic0 = 10,
  Relic1 = 11,
  Relic2 = 12,
  Relic3 = 13,
  Relic4 = 14,
  Charm0 = 15,
  Charm1 = 16,
  Charm2 = 17,
  Charm3 = 18,
}

export interface PlayerEquipmentSummary {
  relicsCount: number;
  maxedRelics: number[];
}
