use nom::{space, alphanumeric, multispace};
use nom::IResult;

use std::str;
use std::vec::Vec;

use def::*;

named!(parse_param <&[u8],(String,String)>,
  chain!(
    key: map_res!(
            alt!(
                alphanumeric |
                delimited!(
                    char!('\"'),
                    alphanumeric,
                    char!('\"')
                )
            ),
            str::from_utf8
         )                                 ~
         space?                            ~
         tag!(":")                         ~
         space?                            ~
    val: map_res!(
            alt!(
                alphanumeric |
                delimited!(
                    char!('\"'),
                    alphanumeric,
                    char!('\"')
                )
            ),
            str::from_utf8
         )                                 ~
         multispace?                       ,
    ||{(key.to_string(), val.to_string())}
  )
);

named!(parse_field <&[u8],(String,String, bool)>,
  chain!(
    key: map_res!(alphanumeric, str::from_utf8) ~
         space?                            ~
         tag!(":")                         ~
         space?                            ~
    val: map_res!(
           take_until_either!(" !\n}"),
           str::from_utf8
         )                                 ~
         space?                            ~
    option: tag!("!")?                     ~
         multispace?                       ,
    ||{(key.to_string(), val.to_string(), if option == None {false} else {true})}
  )
);

named!(parse_object_attributes <&[u8], Vec<(String,String,bool)> >,
    delimited!(
        char!('{'),
        many0!(chain!(
            multispace?                      ~
            result: parse_field,
            ||{result}
        )),
        char!('}')
    )
);

named!(parse_object <(String, Vec<(String, String, bool)>)>,
    chain!(
        tag!("type")                         ~
        space                                ~
        name: map_res!(alphanumeric, str::from_utf8) ~
        multispace?                          ~
        attrs: parse_object_attributes,
        || {(name.to_string(), attrs)}
    )
);

named! (pub parse_all_objects <&[u8], Vec <(String, Vec<(String, String, bool)>)> >,
    many0!(chain!(
        multispace?                          ~
        result: parse_object                 ~
        multispace?,
        ||{result}
    ))
);

named! (parse_query_object <&[u8], QueryObject>,
    chain!(
        multispace?                      ~
        object: map_res!(
                    alt!(
                        alphanumeric |
                        delimited!(
                            char!('\"'),
                            take_until_either!("\""),
                            char!('\"')
                        )
                    ),
                    str::from_utf8
                )                        ~
        space?                           ~
        params: delimited!(
            char!('('),
            many0!(chain!(
                multispace?              ~
                param: parse_param       ~
                multispace?,
                ||{param}
            )),
            char!(')')
        )?                               ~
        space?                           ~
        attributes: delimited!(
            char!('{'),
            many0!(chain!(
                multispace?              ~
                attr: parse_query_object ~ //recursion
                multispace?,
                ||{attr}
            )),
            char!('}')
        )?                                ~
        multispace?,
        ||{QueryObject{name: object.to_string(), params: params, attrs: attributes}}
    )
);

named! (pub parse_query <&[u8], QueryObject>,
    chain!(
        multispace?                              ~
        res: delimited!(
            char!('{'),
            parse_query_object,
            char!('}')
        )                                        ~
        multispace?,
        ||{res}
    )
);

named! (parse_mutation_object <&[u8], MutationObject>,
    chain!(
        multispace?                      ~
        name: map_res!(
            alt!(
                alphanumeric |
                delimited!(
                    char!('\"'),
                    take_until_either!("\""),
                    char!('\"')
                )
            ),
            str::from_utf8
        )                                ~
        space?                           ~
        value: chain! (
            tag!(":")                    ~
            space?                       ~
            res: map_res!(
                alt!(
                    alphanumeric |
                    delimited!(
                        char!('\"'),
                        take_until_either!("\""),
                        char!('\"')
                    )
                ),
                str::from_utf8
            ),
            ||{res.to_string()}
        )?                               ~
        params: delimited!(
            char!('('),
            many0!(chain!(
                multispace?              ~
                res: parse_param         ~
                multispace?,
                ||{res}
            )),
            char!(')')
        )?                               ~
        space?                           ~
        attributes: delimited!(
            char!('{'),
            many0!(chain!(
                multispace?              ~
                res: parse_mutation_object ~ //recursion
                multispace?,
                ||{res}
            )),
            char!('}')
        )?                               ~
        multispace?,
        ||{MutationObject{name: name.to_string(), value: value, params: params, attrs: attributes}}
    )
);

named! (pub parse_mutation_query <&[u8], MutationObject>,
    chain!(
        multispace?                              ~
        res: delimited!(
            char!('{'),
            parse_mutation_object,
            char!('}')
        )                                        ~
        multispace?,
        ||{res}
    )
);


#[test]
fn test_internal_parser_functions(){
    assert_eq!(
        parse_param(&b"id: \"1\"
                    "[..]),
        IResult::Done(&b""[..], {("id".to_string(), "1".to_string())})
    );

    assert_eq!(
        parse_field(&b"id : String!
                    "[..]),
        IResult::Done(&b""[..], {("id".to_string(), "String".to_string(), true)})
    );


    assert_eq!(
        parse_field(&b"id:'1'
                    "[..]),
        IResult::Done(&b""[..], {("id".to_string(), "\'1\'".to_string(), false)})
    );

    assert_eq!(
        parse_field(&b"id:[Object]
                    "[..]),
        IResult::Done(&b""[..], {("id".to_string(), "[Object]".to_string(), false)})
    );

    let cols = IResult::Done(&b""[..], vec![
        {("id".to_string(), "String".to_string(), false)},
        {("name".to_string(), "String".to_string(), false)},
        {("homePlanet".to_string(), "String".to_string(), false)},
        {("list".to_string(), "[String]".to_string(), false)}
    ]);
    assert_eq!(
        parse_object_attributes(&b"{
                    id: String
                    name: String
                    homePlanet: String
                    list: [String]
                 }"[..]),
        cols
    );

    let result = IResult::Done(
        &b""[..],
        ("Human".to_string(),
         vec![
            {("id".to_string(), "String".to_string(), false)},
            {("name".to_string(), "String".to_string(), false)},
            {("homePlanet".to_string(), "String".to_string(), false)}
        ])
    );
    assert_eq!(
        parse_object(
                &b"type Human{
                    id: String
                    name: String
                    homePlanet: String
                }"[..]
        ),
        result
    );
}

#[test]
fn test_get_parser_function(){
    let get_query =
    &b"{
        user (id:1) {
            name
            phone
        }
    }"[..];
    let get_query_data = IResult::Done(&b""[..],
        {QueryObject {
            name:"user".to_string(),
            params: Some(vec![{("id".to_string(), "1".to_string())}]),
            attrs: Some(vec![
                QueryObject {
                    name: "name".to_string(),
                    params: None,
                    attrs: None
                },
                QueryObject {
                    name: "phone".to_string(),
                    params: None,
                    attrs: None
                }
            ])
        }}
    );

    assert_eq!(parse_query(get_query), get_query_data);

    let get_query =
    &b"{
        user (id:\"1\") {
            name
            friends {
              id
              name
            }
        }
    }"[..];

    let get_query_data = IResult::Done(&b""[..],
                                       {QueryObject {
                                           name:"user".to_string(),
                                           params: Some(vec![{("id".to_string(), "1".to_string())}]),
                                           attrs: Some(vec![
                                               QueryObject {
                                                    name: "name".to_string(),
                                                    params: None,
                                                    attrs: None
                                               },
                                               QueryObject {
                                                    name: "friends".to_string(),
                                                    params: None,
                                                    attrs: Some(vec![
                                                        QueryObject {
                                                            name: "id".to_string(),
                                                            params: None,
                                                            attrs: None
                                                        },
                                                        QueryObject {
                                                            name: "name".to_string(),
                                                            params: None,
                                                            attrs: None
                                                        }
                                                    ])
                                                }
                                            ])
                                       }}
    );
    assert_eq!(parse_query(get_query), get_query_data);
}

#[test]
fn test_insert_parser_function(){
    let mut insert_query =
    &b"{
        Human {
            id: 1
            name: Luke
            homePlanet: Char
        }
    }"[..];
    let mut insert_query_data = IResult::Done(&b""[..], {MutationObject {
        name: "Human".to_string(),
        value: None,
        params: None,
        attrs: Some(vec![
                        MutationObject {
                            name: "id".to_string(),
                            value: Some("1".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "name".to_string(),
                            value: Some("Luke".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "homePlanet".to_string(),
                            value: Some("Char".to_string()),
                            params: None,
                            attrs: None
                        }
                    ])
    }});
    assert_eq!(parse_mutation_query(insert_query), insert_query_data);
    insert_query =
    &b"{
        Droid {
            id: 1
            name: \"R2D2\"
            age: 3
            primaryFunction: \"Mechanic\"
        }
    }"[..];
    insert_query_data = IResult::Done(&b""[..], {MutationObject {
        name: "Droid".to_string(),
        value: None,
        params: None,
        attrs: Some(vec![
                        MutationObject {
                            name: "id".to_string(),
                            value: Some("1".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "name".to_string(),
                            value: Some("R2D2".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "age".to_string(),
                            value: Some("3".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "primaryFunction".to_string(),
                            value: Some("Mechanic".to_string()),
                            params: None,
                            attrs: None
                        }
                    ])
    }});
    assert_eq!(parse_mutation_query(insert_query), insert_query_data);

    insert_query =
    &b"{
        Human {
            \"id\": 1
            \"name\": \"Luke\"
            friends {
                Human (
                    \"id\": 2
                    name: Leia
                )
                Human (
                    \"id\": 3
                    name: Han
                )
            }
        }
    }"[..];


    insert_query_data = IResult::Done(&b""[..], {MutationObject {
        name: "Human".to_string(),
        value: None,
        params: None,
        attrs: Some(vec![
                        MutationObject {
                            name: "id".to_string(),
                            value: Some("1".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "name".to_string(),
                            value: Some("Luke".to_string()),
                            params: None,
                            attrs: None
                        },
                        MutationObject {
                            name: "friends".to_string(),
                            value: None,
                            params: None,
                            attrs: Some(vec![
                                MutationObject {
                                    name: "Human".to_string(),
                                    value: None,
                                    params: Some(vec![("id".to_string(), "2".to_string()), ("name".to_string(), "Leia".to_string())]),
                                    attrs: None
                                },
                                MutationObject {
                                    name: "Human".to_string(),
                                    value: None,
                                    params: Some(vec![("id".to_string(), "3".to_string()), ("name".to_string(), "Han".to_string())]),
                                    attrs: None
                                }
                            ])
                        }
                    ])
    }});
    assert_eq!(parse_mutation_query(insert_query), insert_query_data);

}

#[test]
fn test_update_parser_function(){
    let update_query =
    &b"{
        Droid (id:1) {
            age: 4
        }
    }"[..];
    let update_query_data = IResult::Done(&b""[..], {MutationObject {
            name: "Droid".to_string(),
            value: None,
            params: Some(vec![{("id".to_string(), ("1".to_string()))}]),
            attrs: Some(vec![
                MutationObject {
                    name: "age".to_string(),
                    value: Some("4".to_string()),
                    params: None,
                    attrs: None
                }
            ])
    }});
    assert_eq!(parse_mutation_query(update_query), update_query_data);
}

#[test]
fn test_delete_parser_function(){
    let mut delete_query =
    &b"{
        user (id:1)
    }"[..];
    let mut delete_query_data = IResult::Done(&b""[..], {MutationObject {
        name: "user".to_string(),
        value: None,
        params: Some(vec![{("id".to_string(), ("1".to_string()))}]),
        attrs: None
    }});
    assert_eq!(parse_mutation_query(delete_query), delete_query_data);

    delete_query =
    &b"{
        user
    }"[..];
    delete_query_data = IResult::Done(&b""[..], {MutationObject {
        name: "user".to_string(),
        value: None,
        params: None,
        attrs: None
    }});
    assert_eq!(parse_mutation_query(delete_query), delete_query_data);
}