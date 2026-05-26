// <Query> ::= <select-statement> | <create-statement> | <drop-statement> | <alter-statement> | <insert-statement>
// <select-statement> ::= SELECT [column-identifier]{, <column-identifier>} [(, <aggregate-function> | <aggregate)] FROM (<table-identifier>|<statement>) []

use quote::ToTokens;
use syn::spanned::Spanned;

mod kw {
    syn::custom_keyword!(select);
    syn::custom_keyword!(create);
    syn::custom_keyword!(with);
    syn::custom_keyword!(from);
    syn::custom_keyword!(group);
    syn::custom_keyword!(by);
    syn::custom_keyword!(having);
    syn::custom_keyword!(all);
    syn::custom_keyword!(distinct);
    syn::custom_keyword!(join);
    syn::custom_keyword!(on);
}

// fn parse_keyword_insensitive(input: syn::parse::ParseStream, keyword: &str) -> syn::Result<syn::Ident> {
//     let token = input.parse::<syn::Ident>()?;
//     if token.to_string().to_lowercase() != keyword.to_lowercase() {
//         return Err(syn::Error::new(
//             token.span(),
//             format!("expected `{}`", keyword),
//         ));
//     }
//     Ok(token)
// }

fn parse_optional<T: syn::parse::Parse>(input: &syn::parse::ParseStream, keyword: impl syn::parse::Peek) -> syn::Result<Option<T>> {
    if input.peek(keyword) {
        Ok(Some(input.parse::<T>()?))
    }
    else {
        Ok(None)
    }
}

fn parse_till<T: syn::parse::Parse>(input: syn::parse::ParseStream) -> syn::Result<Vec<T>> {
    let mut items: Vec<T> = Vec::new();

    if input.is_empty() {
        return Err(syn::Error::new(input.span(), "unexpected end of input"));
    }

    while !input.is_empty() {
        let item: T = match input.parse() {
            Ok(v) => v,
            Err(e) => {
                if let Err(_) = peek_keyword(&input) {
                    break;
                }
                else {
                    return Err(e);
                }
            }
        };
        items.push(item);

        if input.peek(syn::Token![,]) {
            let _: syn::Token![,] = input.parse()?;
        } else {
            break;
        }
    }

    Ok(items)
}

struct CTE{
    with_token: kw::with,
    name: syn::Ident,
    as_token: syn::Token![as],
    brace_token: syn::token::Brace,
    select_statement: Select,
}
impl syn::parse::Parse for CTE {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            with_token: input.parse()?,
            name: input.parse()?,
            as_token: input.parse()?,
            brace_token: syn::braced!(content in input),
            select_statement: content.parse::<Select>()?,
        })
    }
}
impl quote::ToTokens for CTE {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}
impl CTE {
    fn validate(&self) -> syn::Result<()> {
        self.select_statement.validate()
    }
}

struct Alias {
    as_token: syn::Token![as],
    alias: syn::Ident,
}
impl syn::parse::Parse for Alias {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let as_token = input.parse::<syn::Token![as]>()?;

        if let Err(e) = peek_keyword(&input) {
            return Err(e);
        }

        let alias = input.parse::<syn::Ident>()?;

        Ok(Self {
            as_token,
            alias,
        })
    }
}

fn peek_keyword(input: &syn::parse::ParseStream) -> syn::Result<()> {
    if input.peek(kw::select) | input.peek(kw::with) | input.peek(kw::from)
        | input.peek(kw::group) | input.peek(kw::by) | input.peek(kw::having)
        | input.peek(kw::all) | input.peek(kw::distinct) {
        return Err(syn::Error::new(input.span(), "encountered unexpected keyword"))
    }

    Ok(())
}

struct Aggregate {
    function: syn::Ident,
    paren_token: syn::token::Paren,
    fields: syn::punctuated::Punctuated<syn::Ident, syn::Token![,]>,
}
impl syn::parse::Parse for Aggregate {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Aggregate {
            function: input.parse::<syn::Ident>()?,
            paren_token: syn::parenthesized!(content in input),
            fields: content.parse_terminated(syn::Ident::parse, syn::Token![,])?,
        })
    }
}

struct Column {
    name: syn::Ident,
    alias: Option<Alias>,
}
impl syn::parse::Parse for Column {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            alias: input.parse().ok()
        })
    }
}

enum Selections {
    Aggregate(Aggregate),
    Column(Column),
}
impl syn::parse::Parse for Selections {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if let Err(e) = peek_keyword(&input) {
            return Err(e);
        }
        if input.peek(syn::Ident) && input.peek2(syn::token::Paren) {
            Ok(Selections::Aggregate(input.parse()?))
        }
        else {
            Ok(Selections::Column(input.parse()?))
        }
    }
}

struct GroupBy{
    group_token: kw::group,
    by_token: kw::by,
    grouping: List<Selections>,
}
impl syn::parse::Parse for GroupBy {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self{
            group_token: input.parse()?,
            by_token: input.parse()?,
            grouping: input.parse()?,
        })
    }
}
impl quote::ToTokens for GroupBy {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}

struct Condition(syn::Expr);
impl syn::parse::Parse for Condition {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let condition = match input.parse::<syn::Expr>() {
            Ok(expr) => expr,
            Err(e) => return Err(syn::Error::new(e.span(), "expected a condition")),
        };

        if let syn::Expr::Binary(expr) = &condition {
            match expr.op  {
                syn::BinOp::Eq(_) => Ok(Self(condition)),
                syn::BinOp::Le(_) => Ok(Self(condition)),
                syn::BinOp::Ge(_) => Ok(Self(condition)),
                syn::BinOp::Gt(_) => Ok(Self(condition)),
                syn::BinOp::Lt(_) => Ok(Self(condition)),
                syn::BinOp::Ne(_) => Ok(Self(condition)),
                _ => Err(syn::Error::new(expr.op.span(),"expected `==`,`>` or `<` or `>=` or `<=` or `!=`"))
            }
        }
        else {
            Err(syn::Error::new(condition.span(), "expected binary expression"))
        }
    }
}
impl quote::ToTokens for Condition {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}

struct Filter {
    where_token: syn::Token![where],
    conditions: List<Condition>
}
impl syn::parse::Parse for Filter {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            where_token: input.parse()?,
            conditions: input.parse::<List<Condition>>()?,
        })
    }
}
impl quote::ToTokens for Filter {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}

struct Join {
    join_token: kw::join,
    join_table: syn::Ident,
    alias: Option<Alias>,
    on_token: kw::on,
    condition: List<Condition>
}
impl syn::parse::Parse for Join {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            join_token: input.parse()?,
            join_table: input.parse()?,
            alias: input.parse().ok(),
            on_token: input.parse()?,
            condition: input.parse()?,
        })
    }
}

enum SelectKeywords {
    All(kw::all),
    Distinct(kw::distinct),
}
impl syn::parse::Parse for SelectKeywords {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(kw::all) {
            Ok(Self::All(input.parse()?))
        }
        else if input.peek(kw::distinct) {
            Ok(Self::Distinct(input.parse()?))
        }
        else {
            Err(input.error("expected `all`, `distinct`"))
        }
    }
}

struct Having{
    having_token: kw::having,
    conditions: List<Condition>
}
impl syn::parse::Parse for Having {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self{
            having_token: input.parse()?,
            conditions: input.parse::<List<Condition>>()?,
        })
    }
}
impl quote::ToTokens for Having {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}

struct List<T>(Vec<T>);
impl<T> syn::parse::Parse for List<T>
where T: syn::parse::Parse {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content = match parse_till::<T>(input) {
            Ok(content) => content,
            Err(e) => return Err(e),
        };
        Ok(Self(content))
    }
}

struct FromItem {
    from_token: kw::from,
    source: syn::Ident,
}
impl syn::parse::Parse for FromItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            from_token: input.parse()?,
            source: input.parse()?,
        })
    }
}

struct Select {
    statement: kw::select,
    keywords: Option<SelectKeywords>,
    selections: List<Selections>,
    from_item: FromItem,
    join: Option<Join>,
    group_by: Option<GroupBy>,
    having: Option<Having>,
    filter: Option<Filter>,

}
impl syn::parse::Parse for Select {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            statement: input.parse::<kw::select>()?,
            keywords: input.parse::<SelectKeywords>().ok(),
            selections: input.parse::<List<Selections>>()?,
            from_item: input.parse::<FromItem>()?,
            join: parse_optional(&input, kw::join)?,
            group_by: parse_optional(&input, kw::group)?,
            having: parse_optional(&input, kw::having)?,
            filter: parse_optional(&input, syn::Token![where])?,
        })
    }
}
impl quote::ToTokens for Select {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}
impl Select {
    fn validate(&self) -> syn::Result<()> {
        let has_aggregate = self.selections.0.iter().any(|f| matches!(f, Selections::Aggregate(..)));

        if has_aggregate && self.group_by.is_none() {
            let span = self.selections.0.iter().find_map(|f| match f {
                Selections::Aggregate(aggregate) => Some(aggregate.function.span()),
                _ => None,
            }).unwrap();

            return Err(syn::Error::new(
                span,
                "aggregate functions require a GROUP BY clause",
            ));
        }

        if self.having.is_some() && self.group_by.is_none() {
            return Err(syn::Error::new(
                self.having.as_ref().unwrap().having_token.span(),
                "Having clauses require a GROUP BY clause",
            ))
        }

        if self.group_by.is_some() && !has_aggregate {
            return Err(syn::Error::new(
                self.group_by.as_ref().unwrap().group_token.span(),
                "GROUP BY has no effect without an aggregate function",
            ));
        }

        Ok(())
    }
}

struct Create {

}

impl syn::parse::Parse for Create {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        todo!()
    }
}

impl quote::ToTokens for Create {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {

    }
}
enum Statements {
    CTE(CTE),
    Select(Select),
    Create(Create),
}
impl syn::parse::Parse for Statements {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(kw::select) {
            Ok(Self::Select(input.parse()?))
        }
        else if input.peek(kw::with) {
            Ok(Self::CTE(input.parse()?))
        }
        else if input.peek(kw::create)  {
            Ok(Self::Create(input.parse()?))
        }
        else {
            Err(syn::Error::new(
            input.span(),
            "Not supported statement format"
        ))}
    }
}
impl Statements {
    fn validate(&self) -> syn::Result<()> {
        match self {
            Self::Select(select) => select.validate(),
            Self::CTE(cte) => cte.validate(),
            _ => Ok(())
        }
    }
}

struct Sql {
    statement: Statements,
}

impl syn::parse::Parse for Sql {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            statement: input.parse()?,
        })
    }
}
impl quote::ToTokens for Sql {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Sql { statement} = self;

        match statement {
            Statements::CTE(cte) => {
                let output = {
                    quote::quote! {
                        #cte
                    }
                };
            },
            Statements::Select(s) => {
                let output = {
                    quote::quote! {
                        #s
                    }
                };
                tokens.extend(output)
            }
            Statements::Create(s) => {
                let output = {
                    quote::quote! {
                        #s
                    }
                };
                tokens.extend(output)
            }
        }

    }
}

#[proc_macro]
pub fn sql(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let s = syn::parse_macro_input!(input as Sql);
    if let Err(e)  = s.statement.validate() {
        return e.to_compile_error().into();
    }
    quote::quote!{ #s }.into()
}