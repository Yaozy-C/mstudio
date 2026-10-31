export type WorkContext = {
  view:
    | "drafts"
    | "concepts"
    | "script"
    | "materials"
    | "assets"
    | "storyboard"
    | "film";
  screenplayId?: string;
  paragraphId?: string;
};
