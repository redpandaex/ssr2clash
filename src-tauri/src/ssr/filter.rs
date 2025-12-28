use crate::model::ssr::SsrNode;

/// 过滤节点
///
/// 根据包含关键词和排除关键词过滤节点列表
pub fn filter_nodes(
    nodes: Vec<SsrNode>,
    filter_keywords: &[String],
    exclude_keywords: &[String],
) -> Vec<SsrNode> {
    nodes
        .into_iter()
        .filter(|node| {
            let name = node.remarks.to_lowercase();

            // 如果有包含关键词，必须匹配至少一个
            let include_match = filter_keywords.is_empty()
                || filter_keywords
                    .iter()
                    .any(|keyword| name.contains(&keyword.to_lowercase()));

            // 不能匹配任何排除关键词
            let exclude_match = exclude_keywords
                .iter()
                .any(|keyword| name.contains(&keyword.to_lowercase()));

            include_match && !exclude_match
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_nodes() {
        let nodes = vec![
            SsrNode {
                remarks: "香港节点1".to_string(),
                ..Default::default()
            },
            SsrNode {
                remarks: "美国节点1".to_string(),
                ..Default::default()
            },
            SsrNode {
                remarks: "香港过期节点".to_string(),
                ..Default::default()
            },
        ];

        // 测试包含关键词
        let filtered = filter_nodes(nodes.clone(), &["香港".to_string()], &[]);
        assert_eq!(filtered.len(), 2);

        // 测试排除关键词
        let filtered = filter_nodes(nodes.clone(), &["香港".to_string()], &["过期".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].remarks, "香港节点1");
    }
}
