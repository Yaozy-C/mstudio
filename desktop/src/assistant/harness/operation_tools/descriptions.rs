pub fn description(op: &str) -> &'static str {
    match op {
        "update_generation" => {
            "Save a complete replacement prompt on the existing taskKey. Does not regenerate or change the shot draft."
        }
        "regenerate_generation" => {
            "Create a new run from an existing taskKey, retaining its model, inputs and parameters. Requires an explicit regeneration request."
        }
        "add_node" => {
            "Create a project node with its kind, ID and title. Use only fields permitted for that kind."
        }
        "remove_node" => "Delete the node identified by id.",
        "set_brief" => "Replace the project brief with text.",
        "set_creation" => {
            "Update the project creative intent, essential requirements, preserved decisions or stage."
        }
        "choose_take" => "Choose assetId as the selected take for shot id.",
        "assemble_screenplay" => {
            "Assemble the screenplay identified by id into the timeline using its shots."
        }
        "append_clip" => {
            "Append assetId to the timeline with optional placement, source trim and visual settings. Times are seconds."
        }
        "update_clip" => {
            "Update the timeline clip identified by id. Times are seconds; speed is an absolute playback multiplier."
        }
        "move_clip" => {
            "Move a timeline clip to start (output seconds), optionally changing trackId. allowOverlap explicitly allows overlap."
        }
        "retime_clip" => {
            "Set a clip's playback speed. ripple shifts subsequent clips when requested."
        }
        "slip_clip" => {
            "Shift the clip source range by sourceOffset seconds without changing its output placement."
        }
        "remove_clip" => "Remove the timeline clip identified by id.",
        "set_transition" => "Set the transition from fromClipId into clip id. duration is seconds.",
        "add_track" => "Create a named video or audio timeline track.",
        "update_track" => "Update a track's title, muted or hidden state by id.",
        "add_caption" => "Create a caption with text and output start/end times in seconds.",
        "update_caption" => "Update caption text or output start/end times in seconds by id.",
        "remove_caption" => "Delete the caption identified by id.",
        _ => "Apply this named project operation within the current role's permissions.",
    }
}
