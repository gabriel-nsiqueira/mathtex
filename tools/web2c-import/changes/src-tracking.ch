% Source-tracking change file (mathtex).
%
% Adds a stable, translation-surviving anchor for TOKEN COPIES. Token list
% copies in TeX go through store_new_token(info(SRC)) / fast_store_new_token(
% info(SRC)) -- the macro only sees the token VALUE, so the copied cell loses
% the source provenance of SRC (it is re-stamped with the ambient span). That
% is why a single-token macro argument (e.g. the optional [a] degree of \sqrt)
% drops its source byte-span as expl3 re-tokenises it.
%
% By the C/Rust stage these copies are fully inlined (no name to anchor on), so
% we mark them HERE, at the WEB layer where they are still named macros. The
% empty `src_token_copy(dest,src)' procedure plus its call sites survive
% tangle -> web2c -> c2rust faithfully as `srctokencopy(dest,src)' calls; the
% patcher then replaces the procedure body to carry node_src[src]->node_src[dest].
%
% The anchored lines are byte-identical in tex.web and xetex.web, so this one
% change file serves all three engines (tex, etex, xetex).

@x
@p procedure flush_list(@!p:pointer); {makes list of single-word nodes
  available}
@y
@p procedure src_token_copy(@!dest,@!src:pointer);
  {source-tracking anchor: the patcher replaces this body to carry the source
   span of |src| onto the freshly-copied token cell |dest|. No-op otherwise.}
begin if dest=src then do_nothing; end;
@#
procedure flush_list(@!p:pointer); {makes list of single-word nodes
  available}
@z

@x
    repeat store_new_token(info(t)); incr(m); u:=link(t); v:=s;
@y
    repeat store_new_token(info(t)); src_token_copy(p,t); incr(m); u:=link(t); v:=s;
@z

@x
    begin fast_store_new_token(info(r)); r:=link(r);
@y
    begin fast_store_new_token(info(r)); src_token_copy(p,r); r:=link(r);
@z
