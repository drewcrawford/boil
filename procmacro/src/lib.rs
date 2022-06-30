extern crate proc_macro;
use proc_macro::{TokenStream, TokenTree};
use proc_macro::Delimiter::Parenthesis;

fn parse_associated_type(more_items: &mut proc_macro::token_stream::IntoIter, mut help_item: TokenTree) -> (String,TokenTree) {
    let mut complex_generics = "".to_string();
    //if we get <> in the condition, it is the 'associated type' syntax like <F: Trait<AssociatedType=Bar>`
    //here we need to eat an arbitrary number of tokens until we get balance again
    let mut open_brackets = 1;

    complex_generics += "<";
    loop {
        match more_items.next() {
            None => {
                panic!("Expected more `>` near {}",help_item);
            }
            Some(TokenTree::Punct(p)) if p.to_string() == ">" => {
                open_brackets -= 1;
                complex_generics += ">";
                help_item = TokenTree::Punct(p);
                if open_brackets == 0 { break }
            }
            Some(TokenTree::Punct(p)) if p.to_string() == "<" => {
                open_brackets += 1;
                complex_generics += "<";
                help_item = TokenTree::Punct(p);
            }
            Some(TokenTree::Ident(i)) => {
                complex_generics += &i.to_string();
                complex_generics += " "; //identifiers need space at the end
                help_item = TokenTree::Ident(i);
            }
            Some(other) => {
                complex_generics += &other.to_string();
                help_item = other;
            }
        }
    }
    (complex_generics, help_item)
}

/**
This parses generic arguments like `<'a, A,B,C>`.  This doesn't support conditions on the generics.

# Parameters
* g: pass in the head here, if it looks like `<` we wil begin parsing generics
* more_items: In case we need more items, we will get them here
* help_item: Provides help info in case we need to panic

# Return
1.  String (like `"<'a, A,B, C>"`)
2.  New `help_item`
*/
fn parse_generics_simple(g: Option<TokenTree>,more_items: &mut proc_macro::token_stream::IntoIter, mut help_item: TokenTree) -> (String,TokenTree) { //("<A,B>",help_item)
    let mut imp_generics = "".to_string();
    match g {
        Some(TokenTree::Punct(p)) if p.to_string() == "<" => {
            imp_generics += "<";
            //continue to parse the identifier
            loop {
                let generics = more_items.next();
                // println!("dbg {:?}",generics);
                match generics {
                    Some(TokenTree::Ident(i)) => {
                        imp_generics += &i.to_string();
                        imp_generics += " ";
                        help_item = TokenTree::Ident(i);
                    }
                    Some(TokenTree::Punct(p)) if p.to_string() == ">" => {
                        imp_generics += ">";
                        help_item = TokenTree::Punct(p);
                        break
                    }
                    Some(TokenTree::Punct(p)) if p.to_string() == "'" || p.to_string() == "," || p.to_string() == "=" => {
                        imp_generics += &p.to_string();
                        help_item = TokenTree::Punct(p)
                    }
                    Some(TokenTree::Punct(p)) if p.to_string() == "<" => {
                        // println!("parsing associated item");
                        //this is the 'associated item' style syntax e.g. <Trait<Item=Foo>>
                        let parse = parse_associated_type(more_items, help_item);
                        imp_generics += &parse.0;
                        help_item = parse.1;
                        // println!("parsed {}", parse.0);
                    }
                    other => {
                        panic!("Expected generic arguments near {:?} D", other);
                    }
                }

            }
            assert!(imp_generics.ends_with(">"), "Expected generic arguments near {} E", help_item);
            (imp_generics,help_item)
        }
        Some(other) => {
            //generics are pretty much optional
            ("".to_string(),other)
        }
        None => {
            ("".to_string(),help_item)
        }
    }
}

/**
This parses generic arguments like `<'a,'b: 'a, A: Sync>`.  This supports conditions on the generics.

# Parameters
* g: pass in the head here, if it looks like `<` we wil begin parsing generics
* more_items: In case we need more items to parse, we will get them here
* help_item: Provides help info in case we need to panic

# Return
1.  Complex generics (like `"`<'a,'b: 'a, A: Sync>``)
2.  Simple genericsl (like `"<'a,'b,A>"`)
2.  New `help_item`
 */
fn parse_generics_complex(g: Option<TokenTree>,more_items: &mut proc_macro::token_stream::IntoIter, help_item: TokenTree) -> (String,String,TokenTree) {
    let mut complex_generics = "".to_string();
    let mut simple_generics = "".to_string();
    match g {
        Some(TokenTree::Punct(p)) if p.to_string() == "<" => {
            complex_generics += "<";
            simple_generics += "<";
            //continue to parse the identifier
            let mut last_item = help_item;
            'outer:
            loop {
                let generics = more_items.next();

                match generics {
                    None => {
                        break;
                    }
                    Some(TokenTree::Ident(i)) => {
                        complex_generics += &i.to_string();
                        simple_generics += &i.to_string();
                        last_item = TokenTree::Ident(i);
                    }
                    Some(TokenTree::Punct(p)) if p.to_string() == ">" => {
                        complex_generics += &p.to_string();
                        simple_generics += &p.to_string();
                        last_item = TokenTree::Punct(p);
                        break
                    }
                    Some(TokenTree::Punct(p)) if p.to_string() == "'" || p.to_string() == "," => {
                        complex_generics += &p.to_string();
                        simple_generics += &p.to_string();
                        last_item = TokenTree::Punct(p);
                    }
                    Some(TokenTree::Punct(p) )if p.to_string() == ":" => {
                        //we are parsing a condition.  This only propagates to the complex generics.
                        complex_generics += ":";
                        //continue parsing until we end the condition
                        loop {
                            let next = more_items.next();
                            match next {
                                None => {
                                    assert!(complex_generics.ends_with(","), "Expected `,` near {}",last_item);
                                }
                                Some(TokenTree::Ident(i)) => {
                                    complex_generics += &i.to_string();
                                    last_item = TokenTree::Ident(i);
                                }
                                Some(TokenTree::Punct(p)) if p.to_string() == ">" => {
                                    complex_generics += ">";
                                    simple_generics += ">";
                                    last_item = TokenTree::Punct(p);
                                    break 'outer;
                                }
                                Some(TokenTree::Punct(p)) if p.to_string() == "," => {
                                    complex_generics += ",";
                                    simple_generics += ",";
                                    last_item = TokenTree::Punct(p);
                                    break; //inner!
                                }
                                Some(TokenTree::Punct(p)) if p.to_string() == "<" => {
                                    let parsed = parse_associated_type(more_items, last_item);
                                    complex_generics += &parsed.0;
                                    last_item = parsed.1;

                                }
                                Some(TokenTree::Punct(p)) if p.to_string() == "+" || p.to_string() == ":" || p.to_string() == "'"
                                => {
                                    complex_generics += &p.to_string();
                                    last_item = TokenTree::Punct(p);
                                }
                                Some(other) => {
                                        todo!("{:?}",other)
                                        }
                            }
                        }
                    }
                    other => {
                        panic!("Expected generic arguments near {:?} A", other);
                    }
                }
            }
            assert!(complex_generics.ends_with(">"), "Expected generic arguments near {} C", last_item);
            // println!("parsed {} {}",complex_generics, simple_generics);
            (complex_generics, simple_generics, last_item)
        }
        Some(other) => {
            //generics are pretty much optional
            ("".to_string(),"".to_string(), other)
        }
        None => {
            ("".to_string(), "".to_string(), help_item)
        }
    }
}
fn parse_body(g: Option<TokenTree>, help_item: TokenTree) -> (String,String,String,TokenTree) { //path,vis,imp_generics,new help_item
    let mut path = "".to_string();
    let mut vis = "".to_string();
    let mut imp_generics = "".to_string();
    let mut imp_generics_head = None;
    match g {
        None => {
            panic!("Expected parenthesis near {}",help_item)
        }
        Some(TokenTree::Group(g)) if g.delimiter() == Parenthesis => {
            /*
        Need to parse this group.  It contains members like
        group item Ident { ident: "imp", span: #0 bytes(151..154) }
        group item Punct { ch: ':', spacing: Joint, span: #0 bytes(154..156) }
        group item Punct { ch: ':', spacing: Alone, span: #0 bytes(154..156) }
        group item Ident { ident: "Foo", span: #0 bytes(156..159) }
        Punct { ch: ';', spacing: Alone, span: #0 bytes(160..161) }
         */
            let mut new_help_item = None;
            let mut more_items = g.stream().into_iter();
            for item in &mut more_items {
                // println!("path item {}",item);
                match item {
                    //parse `pub`
                    TokenTree::Ident(i) if i.to_string() == "pub" => {
                        vis += "pub";
                        new_help_item = Some(TokenTree::Ident(i));
                    }
                    //parse the `(crate)` in `pub(crate)`
                    TokenTree::Group(g) if vis == "pub" => {
                        vis += &g.to_string();
                        new_help_item = Some(TokenTree::Group(g));
                    }
                    TokenTree::Ident(i) => {
                        path += &i.to_string();
                        path += " ";
                        new_help_item = Some(TokenTree::Ident(i));
                    }
                    TokenTree::Punct(p) if p.to_string() == ":" => {
                        path += ":";
                        new_help_item = Some(TokenTree::Punct(p));
                    }
                    TokenTree::Punct(p) if p.to_string() == ";" => {
                        new_help_item = Some(TokenTree::Punct(p));
                        break;
                    }
                    TokenTree::Punct(p) if p.to_string() == "<" => {
                        //move on to generics
                        imp_generics_head = Some(TokenTree::Punct(p.clone()));
                        new_help_item = Some(TokenTree::Punct(p));
                        break;
                    }
                    other => {
                        path += &other.to_string();
                        imp_generics_head = Some(other.clone());
                        new_help_item = Some(other);
                    }
                }
            }
            let mut help_item = new_help_item.expect(&format!("Expected the body of a struct near {}",g));
            //try parsing the rest as imp_generics
            //parse as imp_generics
            if let Some(imp_generics_head) = imp_generics_head {
                let r = parse_generics_simple(Some(imp_generics_head), &mut more_items, help_item);
                imp_generics = r.0;
                help_item = r.1;
            }
            assert!(!path.is_empty());

            (path,vis,imp_generics,help_item)
        }

        Some(other) => {
            panic!("Expected the body of a struct near {}",other);
        }
    }
}
struct BoilParse {
    ///The name of our wrapping type
    name: String,
    ///String like <A:Trait,B>
    wrap_generics_complex: String,
    ///String like <A,B>
    wrap_generics_simple: String,
    ///Type we are wrapping.  Does not contain generic parameters.
    imp: String,
    ///String like `<A,B>`
    imp_generics: String,
    ///Visibility specifier, if any
    vis: String,
    ///`where A: B` etc.
    where_clause: String,
}
impl BoilParse {
    fn new(item: TokenStream) -> Self {
        #[derive(PartialEq)]
        enum State {
            Initial,
            Struct,
        }
        let mut state = State::Initial;
        let mut help_item = None;
        let mut item_iter = item.into_iter();
        for item in &mut item_iter {
            match item {
                TokenTree::Ident(i) if i.to_string() == "struct" => {
                    state = State::Struct;
                    help_item = Some(TokenTree::Ident(i));
                    break;
                }
                other => {
                    help_item = Some(other);
                }
            }
        }
        let mut help_item = help_item.expect("#[boil] appears to have no content!");
        assert!(state == State::Struct, "Expected a `struct` keyword before {}",help_item);
        //I think the next item should be an identifier of the type?
        let ident_item_in = item_iter.next();
        let ident;
        match ident_item_in {
            None => {
                panic!("Expected an identifier after {}",help_item);
            }
            Some(TokenTree::Ident(i)) => {
                ident = i.to_string();

            }
            Some(other) => {
                panic!("Expected an identifier instead of {} after {}",other, help_item)
            }
        }
        //Either parenthesis or a generic argument
        let generics_maybe = item_iter.next();


        let r = parse_generics_complex(generics_maybe.clone(), &mut item_iter, help_item);
        let wrap_generics_complex = r.0;
        let wrap_generics_simple = r.1;
        help_item = r.2;

        //If we had no generics, parse generics_maybe.
        //If we had generics, pull the next item.
        let body_head = if wrap_generics_complex.is_empty() { generics_maybe } else { item_iter.next() };
        let p = parse_body(body_head,help_item);
        let path = p.0;
        let vis = p.1;
        let imp_generics = p.2;
        help_item = p.3;

        let mut where_clause = "".to_string();
        //try to parse a where clause
        match item_iter.next() {
            None => {
                panic!("Expected `;` near {}",help_item);
            }
            Some(TokenTree::Ident(i)) if i.to_string() == "where" => {
                where_clause += "where ";
                help_item = TokenTree::Ident(i);
                loop {
                    let item = item_iter.next();
                    match item {
                        None => {
                            panic!("Expected end of where clause near {}",help_item);
                        }
                        Some(TokenTree::Punct(p)) if p.to_string() == ";" => {
                            break;
                        }
                        Some(tree) => {
                            where_clause += &tree.to_string();
                            help_item = tree;
                        }
                    }
                }
            }
            Some(TokenTree::Punct(p)) if p.to_string() == ";" => {
                //ok to leave I guess?
            }
            Some(other) => {
                todo!("{}",other);
            }
        }
        Self {
            name: ident,
            imp: path,
            wrap_generics_simple,
            wrap_generics_complex,
            imp_generics,
            vis: vis,
            where_clause,
        }
    }
    pub fn implement(&self, scoped: bool) -> String {
        let identifier = &self.name;
        let imp = &self.imp;
        let vis = &self.vis;
        let wrap_generics_simple = &self.wrap_generics_simple;
        let wrap_generics_complex = &self.wrap_generics_complex;
        let imp_generics = &self.imp_generics;
        let where_clause = &self.where_clause;
        //we need our own generics, and to tack the 'wrap-generics' on the end
        // let additional_wrap_generics = if wrap_generics_complex.is_empty() { "".to_owned() } else {
        //     //strip the <
        //     let mut chars = wrap_generics_complex.chars();
        //     chars.next().unwrap();
        //     ",".to_owned() + chars.as_str()
        // };
        let mut base_impl = format!("
        //asref
        impl {wrap_generics_complex} AsRef<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn as_ref(&self) -> &{identifier}{wrap_generics_simple} {{
                //safe because we're layout-compatible
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} AsRef<{imp}{imp_generics}> for {identifier}{wrap_generics_simple}  {where_clause} {{
             fn as_ref(&self) -> &{imp}{imp_generics} {{
                //safe because we're layout-compatible
                &self.0
             }}
        }}
        //asmut
        impl {wrap_generics_complex} AsMut<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn as_mut(&mut self) -> &mut {identifier}{wrap_generics_simple} {{
                //safe because we're layout-compatible
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} AsMut<{imp}{imp_generics}> for {identifier}{wrap_generics_simple}  {where_clause} {{
             fn as_mut(&mut self) -> &mut {imp}{imp_generics} {{
                //safe because we're layout-compatible
                &mut self.0
             }}
        }}
        //borrow
        impl {wrap_generics_complex} std::borrow::Borrow<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn borrow(&self) -> &{identifier}{wrap_generics_simple} {{
                //safe because we're layout-compatible
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} std::borrow::Borrow<{imp}{imp_generics}> for {identifier}{wrap_generics_simple} {where_clause} {{
             fn borrow(&self) -> &{imp}{imp_generics} {{
                &self.0
             }}
        }}
        impl {wrap_generics_complex} std::borrow::BorrowMut<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn borrow_mut(&mut self) -> &mut {identifier}{wrap_generics_simple} {{
                //safe because we're layout-compatible
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} std::borrow::BorrowMut<{imp}{imp_generics}> for {identifier}{wrap_generics_simple} {where_clause} {{
             fn borrow_mut(&mut self) -> &mut {imp}{imp_generics} {{
                &mut self.0
             }}
        }}
        //from/into
        impl {wrap_generics_complex} From<{imp}{imp_generics}> for {identifier} {wrap_generics_simple} {where_clause} {{
            fn from(t: {imp}{imp_generics}) -> Self {{
                Self(t)
            }}
        }}
        impl {wrap_generics_complex} From<{identifier} {wrap_generics_simple}> for {imp} {imp_generics} {where_clause} {{
            fn from(t: {identifier} {wrap_generics_simple}) -> {imp} {imp_generics} {{
                t.0
            }}
        }}
        //projections.  Box:
        impl {wrap_generics_complex} From<Box<{imp}{imp_generics}>> for Box<{identifier} {wrap_generics_simple}> {where_clause} {{
            fn from(t: Box<{imp}{imp_generics}>) -> Self {{
                let f = Box::into_raw(t) as *mut {identifier} {wrap_generics_simple};
                //safe because we're layout-compatible
                unsafe {{ Box::from_raw(f) }}
            }}
        }}
        impl {wrap_generics_complex} From<Box<{identifier}{wrap_generics_simple}>> for Box<{imp}{imp_generics}> {where_clause} {{
            fn from(t: Box<{identifier}{wrap_generics_simple}>) -> Self {{
                let f = Box::into_raw(t) as *mut {imp}{imp_generics};
                //safe because we're layout-compatible
                unsafe {{ Box::from_raw(f) }}
            }}
        }}
        //Pin:
        impl {wrap_generics_complex} From<core::pin::Pin<&{imp}{imp_generics}>> for core::pin::Pin<&{identifier} {wrap_generics_simple}> {where_clause} {{
            fn from(t: core::pin::Pin<&{imp}{imp_generics}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core::pin::Pin::into_inner_unchecked(t) as &_ as *const _ as *const {identifier} {wrap_generics_simple};
                    core::pin::Pin::new_unchecked(&*f)
                }}

            }}
        }}
        //PinMut:
        impl {wrap_generics_complex} From<core::pin::Pin<&mut {imp}{imp_generics}>> for core::pin::Pin<&mut {identifier} {wrap_generics_simple}> {where_clause} {{
            fn from(t: core::pin::Pin<&mut {imp}{imp_generics}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core::pin::Pin::into_inner_unchecked(t) as &mut _ as *mut _ as *mut {identifier} {wrap_generics_simple};
                    core::pin::Pin::new_unchecked(&mut *f)
                }}

            }}
        }}
        //other direction Pin:
        impl {wrap_generics_complex} From<core::pin::Pin<&{identifier}{wrap_generics_simple}>> for core::pin::Pin<&{imp}{imp_generics}> {where_clause} {{
            fn from(t: core::pin::Pin<&{identifier}{wrap_generics_simple}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core::pin::Pin::into_inner_unchecked(t) as *const _ as *const {imp}{imp_generics};
                    core::pin::Pin::new_unchecked(&*f)
                }}
            }}
        }}
        //PinMut
        impl {wrap_generics_complex} From<core::pin::Pin<&mut {identifier}{wrap_generics_simple}>> for core::pin::Pin<&mut {imp}{imp_generics}> {where_clause} {{
            fn from(t: core::pin::Pin<&mut {identifier}{wrap_generics_simple}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core::pin::Pin::into_inner_unchecked(t) as *mut _ as *mut {imp}{imp_generics};
                    core::pin::Pin::new_unchecked(&mut *f)
                }}
            }}
        }}

        //Arc and Rc projections
        impl {wrap_generics_complex} {identifier}{wrap_generics_simple} {where_clause} {{
            /**
            Converts from an [std::sync::Arc] of underlying type to an [std::sync::Arc] of the wrapper.

            This is a zero-cost abstraction. */
            {vis} fn from_arc(arc: std::sync::Arc<{imp}{imp_generics}>) -> std::sync::Arc<{identifier}{wrap_generics_simple}> {{
                let f = std::sync::Arc::into_raw(arc) as *mut {identifier} {wrap_generics_simple};
                //safe because we're layout-compatible
                unsafe {{ std::sync::Arc::from_raw(f) }}
            }}
            /**
            Converts from an [std::sync::Arc] of wrapper type to an [std::sync::Arc] of the underlying type.

            This is a zero-cost abstraction. */
            {vis} fn to_arc(self: std::sync::Arc<Self>) -> std::sync::Arc<{imp}{imp_generics}> {{
                let f = std::sync::Arc::into_raw(self) as *mut {imp} {imp_generics};
                //safe because we're layout-compatible
                unsafe {{ std::sync::Arc::from_raw(f) }}
            }}
            /**
            Converts from an [std::rc::Rc] of underlying type to an [std::rc::Rc] of the wrapper.

            This is a zero-cost abstraction. */
            {vis} fn from_rc(rc: std::rc::Rc<{imp}{imp_generics}>) -> std::rc::Rc<{identifier}{wrap_generics_simple}> {{
                let f = std::rc::Rc::into_raw(rc) as *mut {identifier} {wrap_generics_simple};
                //safe because we're layout-compatible
                unsafe {{ std::rc::Rc::from_raw(f) }}
            }}
            /**
            Converts from an [std::rc::Rc] of wrapper type to an [std::rc::Rc] of the underlying type.

            This is a zero-cost abstraction. */
            {vis} fn to_rc(self: std::rc::Rc<Self>) -> std::rc::Rc<{imp}{imp_generics}> {{
                let f = std::rc::Rc::into_raw(self) as *mut {imp} {imp_generics};
                //safe because we're layout-compatible
                unsafe {{ std::rc::Rc::from_raw(f) }}
            }}
        }}

        //Result projections
        impl {wrap_generics_complex} {identifier} {wrap_generics_simple} {where_clause} {{
            /**
            Converts from Result with value of underlying type, into Result of wrapped type.

            If necessary, also converts Error between any types that can be converted with [Into], including between wrapped or underlying types.*/
            {vis} fn from_result<T: Into<Self>,E: Into<ErrWrapped>,ErrWrapped>(r: Result<T,E>) -> Result<Self,ErrWrapped> {{
                r.map(|t| t.into()).map_err(|e| e.into())
            }}
            /**
            Converts from Result with value of wrapped type, into Result of underlying type.

            If necessary, also converts Error between any types that can be converted with [Into], including between wrapped or underlying types.*/
            {vis} fn into_result<E: Into<EUnwrapped>,EUnwrapped>(r: Result<Self,E>) -> Result<{imp}{imp_generics},EUnwrapped> {{
                r.map(|t| t.into()).map_err(|e| e.into())
            }}
        }}

        ");
        if !scoped {
            base_impl += &format!(
                "        //deref and derefmut
        impl {wrap_generics_complex} std::ops::Deref for {identifier}{wrap_generics_simple} {where_clause} {{
            type Target = {imp}{imp_generics};
            fn deref(&self) -> &Self::Target {{
                &self.0
            }}
        }}
        impl {wrap_generics_complex} std::ops::DerefMut for {identifier} {wrap_generics_simple} {where_clause} {{
            fn deref_mut(&mut self) -> &mut Self::Target {{
                &mut self.0
            }}
        }}"
            );
        }
        base_impl
    }
    fn implement_unsized(&self,scoped: bool) -> String {
        let wrap_generics_complex = &self.wrap_generics_complex;
        let identifier = &self.name;
        let where_clause = &self.where_clause;
        let imp = &self.imp;
        let imp_generics = &self.imp_generics;
        let wrap_generics_simple = &self.wrap_generics_simple;
        let vis = &self.vis;
        let mut impl_text = format!("
        impl {wrap_generics_complex} AsRef<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn as_ref(&self) -> &{identifier}{wrap_generics_simple} {{
                //safe because identifier is layout-compatible with `dyn Trait`
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} AsRef<{imp}{imp_generics}> for {identifier}{wrap_generics_simple}  {where_clause} {{
             fn as_ref(&self) -> &({imp}{imp_generics} + 'static) {{
                &self.0
             }}
        }}
        //asmut
        impl {wrap_generics_complex} AsMut<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn as_mut(&mut self) -> &mut {identifier}{wrap_generics_simple} {{
                //safe because identifier is layout-compatible with `dyn Trait`
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} AsMut<{imp}{imp_generics}> for {identifier}{wrap_generics_simple}  {where_clause} {{
             fn as_mut(&mut self) -> &mut ({imp}{imp_generics} + 'static) {{
                &mut self.0
             }}
        }}
        //borrow
        impl {wrap_generics_complex} std::borrow::Borrow<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn borrow(&self) -> &{identifier}{wrap_generics_simple} {{
                //safe because identifier is layout-compatible with `dyn Trait`
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} std::borrow::Borrow<{imp}{imp_generics}> for {identifier}{wrap_generics_simple} {where_clause} {{
             fn borrow(&self) -> &({imp}{imp_generics} + 'static) {{
                &self.0
             }}
        }}
        impl {wrap_generics_complex} std::borrow::BorrowMut<{identifier}{wrap_generics_simple}> for {imp}{imp_generics} {where_clause} {{
             fn borrow_mut(&mut self) -> &mut {identifier}{wrap_generics_simple} {{
                //safe because identifier is layout-compatible with `dyn Trait`
                unsafe {{ std::mem::transmute(self) }}
             }}
        }}
        impl {wrap_generics_complex} std::borrow::BorrowMut<{imp}{imp_generics}> for {identifier}{wrap_generics_simple} {where_clause} {{
             fn borrow_mut(&mut self) -> &mut ({imp}{imp_generics} + 'static) {{
                &mut self.0
             }}
        }}
        //from/into
        impl {wrap_generics_complex} From<&{imp}{imp_generics}> for &{identifier} {wrap_generics_simple} {where_clause} {{
            fn from(t: &{imp}{imp_generics}) -> Self {{
                unsafe {{ &*(t as *const _ as *const _) }}
            }}
        }}
        impl {wrap_generics_complex} From<&{identifier} {wrap_generics_simple}> for &{imp} {imp_generics} {where_clause} {{
            fn from(t: &{identifier} {wrap_generics_simple}) -> Self {{
                //transmute required here since wrapper not known to conform to payload type
                unsafe {{ std::mem::transmute(t) }}
            }}
        }}
        impl {wrap_generics_complex} From<&mut {imp}{imp_generics}> for &mut {identifier} {wrap_generics_simple} {where_clause} {{
            fn from(t: &mut {imp}{imp_generics}) -> Self {{
                unsafe {{ &mut *(t as *mut _ as *mut _) }}
            }}
        }}
        impl {wrap_generics_complex} From<&mut {identifier} {wrap_generics_simple}> for &mut {imp} {imp_generics} {where_clause} {{
            fn from(t: &mut {identifier} {wrap_generics_simple}) -> Self {{
                //transmute required here since wrapper not known to conform to payload type
                unsafe {{ std::mem::transmute(t) }}
            }}
        }}

        //projections.  Box:
        impl {wrap_generics_complex} From<Box<{imp}{imp_generics}>> for Box<{identifier} {wrap_generics_simple}> {where_clause} {{
            fn from(t: Box<{imp}{imp_generics}>) -> Self {{
                let f = Box::into_raw(t) as *mut {identifier} {wrap_generics_simple};
                //safe because we're layout-compatible
                unsafe {{ Box::from_raw(f) }}
            }}
        }}
        impl {wrap_generics_complex} From<Box<{identifier}{wrap_generics_simple}>> for Box<{imp}{imp_generics}> {where_clause} {{
            fn from(t: Box<{identifier}{wrap_generics_simple}>) -> Self {{
                let f = Box :: into_raw(t);
                unsafe {{
                    let g: *mut {imp}{imp_generics} = std::mem::transmute(f);
                    Box :: from_raw(g)
                }}
            }}
        }}
        //Pin:
        impl {wrap_generics_complex} From<core::pin::Pin<&{imp}{imp_generics}>> for core::pin::Pin<&{identifier} {wrap_generics_simple}> {where_clause} {{
            fn from(t: core::pin::Pin<&{imp}{imp_generics}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core::pin::Pin::into_inner_unchecked(t) as &_ as *const _ as *const {identifier} {wrap_generics_simple};
                    core::pin::Pin::new_unchecked(&*f)
                }}

            }}
        }}
        //PinMut:
        impl {wrap_generics_complex} From<core::pin::Pin<&mut {imp}{imp_generics}>> for core::pin::Pin<&mut {identifier} {wrap_generics_simple}> {where_clause} {{
            fn from(t: core::pin::Pin<&mut {imp}{imp_generics}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core::pin::Pin::into_inner_unchecked(t) as &mut _ as *mut _ as *mut {identifier} {wrap_generics_simple};
                    core::pin::Pin::new_unchecked(&mut *f)
                }}

            }}
        }}

        //other direction Pin:
        impl {wrap_generics_complex} From<core::pin::Pin<&{identifier}{wrap_generics_simple}>> for core::pin::Pin<&{imp}{imp_generics}> {where_clause} {{
            fn from(t: core::pin::Pin<&{identifier}{wrap_generics_simple}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core :: pin :: Pin :: into_inner_unchecked(t);
                    //transmute required because wrapper not known to implement dyn trait
                    let g = std::mem::transmute(f);
                    core :: pin :: Pin :: new_unchecked(g)
                }}
            }}
        }}
        //PinMut
        impl {wrap_generics_complex} From<core::pin::Pin<&mut {identifier}{wrap_generics_simple}>> for core::pin::Pin<&mut {imp}{imp_generics}> {where_clause} {{
            fn from(t: core::pin::Pin<&mut {identifier}{wrap_generics_simple}>) -> Self {{
                //safe because we're layout-compatible
                unsafe {{
                    let f = core :: pin :: Pin :: into_inner_unchecked(t);
                    //transmute required because wrapper not known to implement dyn trait
                    let g = std::mem::transmute(f);
                    core :: pin :: Pin :: new_unchecked(g)
                }}
            }}
        }}
        //Arc and Rc projections
        impl {wrap_generics_complex} {identifier}{wrap_generics_simple} {where_clause} {{
            /**
            Converts from an [std::sync::Arc] of underlying type to an [std::sync::Arc] of the wrapper.

            This is a zero-cost abstraction. */
            {vis} fn from_arc(arc: std::sync::Arc<{imp}{imp_generics}>) -> std::sync::Arc<{identifier}{wrap_generics_simple}> {{
                let f = std::sync::Arc::into_raw(arc) as *mut {identifier} {wrap_generics_simple};
                //safe because we're layout-compatible
                unsafe {{ std::sync::Arc::from_raw(f) }}
            }}
            /**
            Converts from an [std::sync::Arc] of wrapper type to an [std::sync::Arc] of the underlying type.

            This is a zero-cost abstraction. */
            {vis} fn to_arc(self: std::sync::Arc<Self>) -> std::sync::Arc<{imp}{imp_generics}> {{
                let f = std::sync::Arc::into_raw(self) as *mut {imp} {imp_generics};
                //safe because we're layout-compatible
                unsafe {{ std::sync::Arc::from_raw(f) }}
            }}
            /**
            Converts from an [std::rc::Rc] of underlying type to an [std::rc::Rc] of the wrapper.

            This is a zero-cost abstraction. */
            {vis} fn from_rc(rc: std::rc::Rc<{imp}{imp_generics}>) -> std::rc::Rc<{identifier}{wrap_generics_simple}> {{
                let f = std::rc::Rc::into_raw(rc) as *mut {identifier} {wrap_generics_simple};
                //safe because we're layout-compatible
                unsafe {{ std::rc::Rc::from_raw(f) }}
            }}
            /**
            Converts from an [std::rc::Rc] of wrapper type to an [std::rc::Rc] of the underlying type.

            This is a zero-cost abstraction. */
            {vis} fn to_rc(self: std::rc::Rc<Self>) -> std::rc::Rc<{imp}{imp_generics}> {{
                let f = std::rc::Rc::into_raw(self) as *mut {imp} {imp_generics};
                //safe because we're layout-compatible
                unsafe {{ std::rc::Rc::from_raw(f) }}
            }}
        }}

        //Result projections
        impl {wrap_generics_complex} {identifier} {wrap_generics_simple} {where_clause} {{
            /**
            Converts from Result with value of underlying type, into Result of wrapped type.

            If necessary, also converts Error between any types that can be converted with [Into], including between wrapped or underlying types.*/
            {vis} fn from_result<E: Into<ErrWrapped>,ErrWrapped>    (r : Result <&{imp}{imp_generics}, E >) -> Result <&Self, ErrWrapped > {{
                r.map(|t| t.into()).map_err(|e| e.into())
            }}
            /**
            Converts from Result with value of wrapped type, into Result of underlying type.

            If necessary, also converts Error between any types that can be converted with [Into], including between wrapped or underlying types.*/
            {vis} fn into_result<E: Into<EUnwrapped>,EUnwrapped>(r : Result < &Self, E >) -> Result < &{imp}{imp_generics}, EUnwrapped > {{
                r.map(|t| t.into()).map_err(|e| e.into())
            }}

            /**
            Converts from Result with value of underlying type, into Result of wrapped type.

            If necessary, also converts Error between any types that can be converted with [Into], including between wrapped or underlying types.*/
            {vis} fn from_result_mut<E: Into<ErrWrapped>,ErrWrapped>    (r : Result <&mut {imp}{imp_generics}, E >) -> Result <&mut Self, ErrWrapped > {{
                r.map(|t| t.into()).map_err(|e| e.into())
            }}
            /**
            Converts from Result with value of wrapped type, into Result of underlying type.

            If necessary, also converts Error between any types that can be converted with [Into], including between wrapped or underlying types.*/
            {vis} fn into_result_mut<E: Into<EUnwrapped>,EUnwrapped>(r : Result < &mut Self, E >) -> Result < &mut {imp}{imp_generics}, EUnwrapped > {{
                r.map(|t| t.into()).map_err(|e| e.into())
            }}
        }}


        ");
        if !scoped {
            impl_text += &format!("
        //deref and derefmut
        impl {wrap_generics_complex} std::ops::Deref for {identifier}{wrap_generics_simple} {where_clause} {{
            type Target = {imp}{imp_generics};
            fn deref(&self) -> &Self::Target {{
                &self.0
            }}
        }}
        impl {wrap_generics_complex} std::ops::DerefMut for {identifier} {wrap_generics_simple} {where_clause} {{
            fn deref_mut(&mut self) -> &mut Self::Target {{
                &mut self.0
            }}
        }}");
        }
        impl_text
    }
}


#[proc_macro_attribute]
pub fn boil(attr: TokenStream, item: TokenStream) -> TokenStream {
    let scoped = match attr.into_iter().find(|p| p.to_string() == "scoped") {
        Some(_) => true,
        None => false,
    };

    // println!("dbg boil");
    //we require types to be repr-transparent
    let mut code: TokenStream = "#[repr(transparent)]\n".parse().unwrap();
    let parse = BoilParse::new(item.clone());
    code.extend(item);
    let parsed_implementation: TokenStream = parse.implement(scoped).parse().unwrap();
    code.extend(parsed_implementation);
    // println!("will emit {}",code);
    code
}

#[proc_macro_attribute]
pub fn boil_unsized(attr: TokenStream, item: TokenStream) -> TokenStream {
    let scoped = match attr.into_iter().find(|p| p.to_string() == "scoped") {
        Some(_) => true,
        None => false,
    };
    let parse = BoilParse::new(item.clone());
    let mut code: TokenStream = "#[repr(transparent)]\n".parse().unwrap();
    code.extend(item);
    let parsed_implementation: TokenStream = parse.implement_unsized(scoped).parse().unwrap();
    code.extend(parsed_implementation);
    // println!("emit {}",parsed_implementation);
    code
}

#[proc_macro_derive(Display)]
pub fn display(item: TokenStream) -> TokenStream {
    let parsed = BoilParse::new(item);
    let wrap_generics_complex = parsed.wrap_generics_complex;
    let wrap_generics_simple = parsed.wrap_generics_simple;
    let where_clause = parsed.where_clause;
    let identifier = parsed.name;
    format!("
        impl {wrap_generics_complex} std::fmt::Display for {identifier} {wrap_generics_simple} {where_clause} {{
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {{
                std::fmt::Display::fmt(&self.0,formatter)
            }}
        }}
    ").parse().unwrap()
}

#[proc_macro_derive(Error)]
pub fn error(item: TokenStream) -> TokenStream {
    let parsed = BoilParse::new(item);
    let wrap_generics_complex = parsed.wrap_generics_complex;
    let wrap_generics_simple = parsed.wrap_generics_simple;
    let where_clause = parsed.where_clause;

    let identifier = parsed.name;
    let code =
    format!("
        impl {wrap_generics_complex} std::error::Error for {identifier} {wrap_generics_simple} {where_clause} {{
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {{ self.0.source() }}
            fn description(&self) -> &str {{ self.0.description() }}
            fn cause(&self) -> Option<&dyn std::error::Error> {{ self.0.cause() }}
        }}
    ");
    // println!("emitting {}",code);
    code.parse().unwrap()
}