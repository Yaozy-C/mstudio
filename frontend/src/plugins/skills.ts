export type CreativeSkill = {
  id: string;
  name: string;
  description: string;
  available: boolean;
  enabled: boolean;
  path: string;
};
export type SkillPage = {
  skill: string;
  path: string;
  text: string;
  nextOffset: number | null;
};
