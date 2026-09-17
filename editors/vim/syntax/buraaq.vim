# Vim filetype for Buraaq (.bq)
# Install: copy ftdetect + syntax into ~/.vim/ or use as a pathogen bundle under editors/vim

if exists("b:current_syntax")
  finish
endif

syntax keyword bqKeyword async await break const continue copy defer drop else elif enum extern false fn for give if impl in match module mut parallel pub raise ref return self spawn struct throws trait true type unsafe use void while as test bench expect ok err some none not new
syntax keyword bqType int i32 i64 u32 u64 float f32 f64 bool text bytes
syntax match bqComment "#.*$"
syntax region bqBlockComment start=/"""/ end=/"""/
syntax region bqString start=/"/ skip=/\\./ end=/"/ contains=bqInterp
syntax region bqInterp start=/{/ end=/}/ contained contains=TOP
syntax match bqNumber /\v<0[xX][0-9a-fA-F_]+|0[bB][01_]+|\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d+)?>/

highlight default link bqKeyword Keyword
highlight default link bqType Type
highlight default link bqComment Comment
highlight default link bqBlockComment Comment
highlight default link bqString String
highlight default link bqNumber Number

let b:current_syntax = "buraaq"
