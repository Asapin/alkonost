use std::collections::HashSet;

pub struct VideoList {
    pub streams: HashSet<String>,
}

mod custom_deser_impls {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct Outer {
        contents: ListContent,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct ListContent {
        #[serde(rename = "twoColumnBrowseResultsRenderer")]
        renderer: TwoColumnRenderer,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct TwoColumnRenderer {
        tabs: Vec<Tabs>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    enum Tabs {
        TabRenderer { content: Option<TabContent> },
        ExpandableTabRenderer {},
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct TabContent {
        section_list_renderer: SectionList,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct SectionList {
        contents: Vec<SectionContent>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct SectionContent {
        item_section_renderer: ItemSection,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct ItemSection {
        contents: Vec<ItemSectionContent>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct ItemSectionContent {
        shelf_renderer: Option<ShelfRenderer>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct ShelfRenderer {
        content: ShelfContent,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    enum ShelfContent {
        GridRenderer { items: Vec<GridShelfItem> },
        HorizontalListRenderer { items: Vec<GridShelfItem> },
        VerticalListRenderer { items: Vec<ShelfItem> },
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct GridShelfItem {
        grid_video_renderer: VideoRenderer,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct ShelfItem {
        video_renderer: VideoRenderer,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct VideoRenderer {
        video_id: String,
        published_time_text: Option<PublishedTimeText>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all(deserialize = "camelCase"))]
    struct PublishedTimeText {}

    impl<'de> Deserialize<'de> for VideoList {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            impl From<GridShelfItem> for Option<String> {
                fn from(item: GridShelfItem) -> Self {
                    match item.grid_video_renderer.published_time_text {
                        Some(_) => None,
                        None => Some(item.grid_video_renderer.video_id),
                    }
                }
            }

            impl From<ShelfItem> for Option<String> {
                fn from(item: ShelfItem) -> Self {
                    match item.video_renderer.published_time_text {
                        Some(_) => None,
                        None => Some(item.video_renderer.video_id),
                    }
                }
            }

            let outer = Outer::deserialize(deserializer)?;
            let streams = outer
                .contents
                .renderer
                .tabs
                .into_iter()
                .filter_map(|tab| match tab {
                    Tabs::TabRenderer { content } => content,
                    Tabs::ExpandableTabRenderer {} => None,
                })
                .flat_map(|tab_content| tab_content.section_list_renderer.contents)
                .flat_map(|section_content| section_content.item_section_renderer.contents)
                .filter_map(|item_section| item_section.shelf_renderer)
                .map(|shelf_renderer: ShelfRenderer| {
                    let streams: HashSet<String> = match shelf_renderer.content {
                        ShelfContent::GridRenderer { items }
                        | ShelfContent::HorizontalListRenderer { items } => {
                            items.into_iter().filter_map(|item| item.into()).collect()
                        }
                        ShelfContent::VerticalListRenderer { items } => {
                            items.into_iter().filter_map(|item| item.into()).collect()
                        }
                    };
                    streams
                })
                .flatten()
                .collect::<HashSet<String>>();

            Ok(VideoList { streams })
        }
    }
}
