use redis_nav::redis_client::RedisType;
use redis_nav::tree::TreeBuilder;

#[test]
fn test_single_delimiter() {
    let builder = TreeBuilder::new(vec![':']);
    let keys = vec![
        ("user:1:name".to_string(), RedisType::String),
        ("user:1:email".to_string(), RedisType::String),
        ("user:2:name".to_string(), RedisType::String),
    ];

    let tree = builder.build(&keys);

    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].name, "user");
    assert_eq!(tree[0].children.len(), 2); // user:1 and user:2
}

#[test]
fn test_multiple_delimiters() {
    let builder = TreeBuilder::new(vec![':', '/']);
    let keys = vec![
        ("user:1:name".to_string(), RedisType::String),
        ("api/v1/users".to_string(), RedisType::String),
    ];

    let tree = builder.build(&keys);

    assert_eq!(tree.len(), 2); // "user" and "api"
}

#[test]
fn test_empty_keys() {
    let builder = TreeBuilder::new(vec![':']);
    let keys: Vec<(String, RedisType)> = vec![];

    let tree = builder.build(&keys);

    assert!(tree.is_empty());
}
