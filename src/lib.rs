#![recursion_limit = "128"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate proc_macro2;
extern crate regex;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{DeriveInput, Field, Ident};

#[proc_macro_attribute]
pub fn findable_by(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut string_input = input.to_string();
    let string_args = args.to_string();
    let ast: DeriveInput = syn::parse(input).unwrap();
    let fields: Vec<Field> = match ast.data {
        syn::Data::Enum(..) => panic!("#[findable_by] cannot be used with enums"),
        syn::Data::Union(..) => panic!("#[findable_by] cannot be used with unions"),
        syn::Data::Struct(ref body) => body.fields.iter().map(|f| f.clone()).collect(),
    };
    let struct_attributes = string_args.replace(" ", "").replace("\"", "");
    let struct_attributes: Vec<&str> = struct_attributes.split(",").collect();
    let struct_name = ast.ident;

    for struct_attribute in struct_attributes {
        let func = gen_find_by_func(
            &struct_name.to_string(),
            &string_input.clone(),
            struct_attribute,
            &fields,
        );

        string_input.push_str(&func);
    }

    string_input.parse().unwrap()
}

fn gen_find_by_func(
    struct_name: &str,
    string_input: &str,
    struct_attribute: &str,
    fields: &Vec<Field>,
) -> String {
    let field: Vec<&Field> = fields
        .iter()
        .filter(|f| f.ident.clone().unwrap().to_string() == struct_attribute)
        .collect();

    if field.len() > 0 {
        let field = field[0];
        let attr_type = &field.ty;

        let struct_name = Ident::new(&format!("{}", struct_name), Span::call_site());
        let func_name = Ident::new(&format!("find_by_{}", struct_attribute), Span::call_site());
        let all_func_name = Ident::new(
            &format!("find_all_by_{}", struct_attribute),
            Span::call_site(),
        );
        let struct_attribute = Ident::new(&struct_attribute, Span::call_site());
        let struct_attribute_col =
            Ident::new(&format!("{}_col", struct_attribute), Span::call_site());
        let table_name = Ident::new(
            &get_table_name(string_input.to_string().clone()),
            Span::call_site(),
        );
        let func = quote! {
            impl #struct_name {
                pub fn #func_name(attr: & #attr_type, conn: &PgConnection) -> Option<#struct_name> {
                    use crate::schema::#table_name::dsl::#struct_attribute as #struct_attribute_col;

                    match #table_name::table.filter(#struct_attribute_col.eq(attr)).first(conn) {
                        Ok(res) => Some(res),
                        Err(_) => None,
                    }
                }

                pub fn #all_func_name(attr: & #attr_type, conn: &PgConnection) -> Result<Vec<#struct_name>, ::diesel::result::Error> {
                    use crate::schema::#table_name::dsl::#struct_attribute as #struct_attribute_col;

                    #table_name::table.filter(#struct_attribute_col.eq(attr)).get_results(conn)
                }
            }
        };

        func.to_string()
    } else {
        panic!(
            "Attribute {} not found in {}",
            struct_attribute, struct_name
        );
    }
}

fn get_table_name(input: String) -> String {
    use regex::Regex;

    let re = Regex::new(r###"#\[table_name = "(.*)"\]"###).unwrap();
    let table_name_attr = input
        .lines()
        .skip_while(|line| !line.trim_start().starts_with("#[table_name ="))
        .next()
        .expect("Struct must be annotated with #[table_name = \"...\"]");

    if let Some(table_name) = re.captures(table_name_attr).unwrap().get(1) {
        table_name.as_str().to_string()
    } else {
        panic!("Malformed table_name attribute");
    }
}
