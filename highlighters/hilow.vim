" Vim syntax file for the HiLow language
" Language:   HiLow (compiles to C)
" Maintainer: Matthew C. Tedder
" Built from src/lexer/mod.rs keyword + operator table.

if exists("b:current_syntax")
  finish
endif

" ---------------------------------------------------------------------------
" Comments
" ---------------------------------------------------------------------------
syn keyword hilowTodo contained TODO FIXME XXX NOTE
syn match   hilowComment "//.*$" contains=hilowTodo
syn region  hilowComment start="/\*" end="\*/" contains=hilowTodo

" ---------------------------------------------------------------------------
" Mode / structural keywords
" ---------------------------------------------------------------------------
syn keyword hilowMode      high low
syn keyword hilowKeyword   function program module import export from
syn keyword hilowKeyword   let return defer
syn keyword hilowKeyword   this is

" Control flow
syn keyword hilowConditional if else match switch case default when
syn keyword hilowRepeat      for in while loop break continue

" Logical operators (word form)
syn keyword hilowOperator and or not

" Async / concurrency
syn keyword hilowKeyword   async

" Memory-mode declarators
syn keyword hilowStorage   arena heap stack manual weak shared stealth

" Formal-verification vocabulary
syn keyword hilowVerify    requires ensures invariant decreases excluding

" The reactive watcher keyword
syn keyword hilowWatcher   watcher

" Watcher modifiers — contextual; highlighted wherever they appear.
" (changed/assigned/deep/added/removed/moved are not reserved keywords in the
"  lexer, but flagging them aids reading watcher subscriptions.)
syn keyword hilowModifier  changed assigned deep added removed moved

" The 'unknown' error-type keyword
syn keyword hilowKeyword   unknown

" ---------------------------------------------------------------------------
" Types
" ---------------------------------------------------------------------------
syn keyword hilowType i8 i16 i32 i64 i128
syn keyword hilowType u8 u16 u32 u64 u128 usize
syn keyword hilowType f32 f64
syn keyword hilowType bool string nothing

" ---------------------------------------------------------------------------
" Constants / literals
" ---------------------------------------------------------------------------
syn keyword hilowBoolean true false

" Currency keywords (money literals like 100USD)
syn keyword hilowCurrency USD EUR GBP JPY CAD AUD CHF CNY

" Numbers (integer, float, and money/duration suffixed forms)
syn match hilowNumber  "\<\d\+\>"
syn match hilowFloat   "\<\d\+\.\d\+\>"
" Money: digits immediately followed by a currency code, e.g. 100USD, 9.99EUR
syn match hilowMoney   "\<\d\+\(\.\d\+\)\?\(USD\|EUR\|GBP\|JPY\|CAD\|AUD\|CHF\|CNY\)\>"
" Duration: digits + time suffix, e.g. 5s, 100ms, 2h (common suffixes)
syn match hilowDuration "\<\d\+\(ns\|us\|ms\|s\|m\|h\|d\)\>"

" ---------------------------------------------------------------------------
" Strings  (regular "..." and f-strings f"...{expr}...")
" ---------------------------------------------------------------------------
syn match  hilowEscape   contained "\\."
" f-string interpolation: { ... } inside an f-string
syn region hilowInterp   contained matchgroup=hilowInterpDelim start="{" end="}" contains=TOP
syn region hilowString   start=+"+ skip=+\\"+ end=+"+ contains=hilowEscape
syn region hilowFString  matchgroup=hilowFStringPrefix start=+f"+ skip=+\\"+ end=+"+ contains=hilowEscape,hilowInterp

" ---------------------------------------------------------------------------
" Operators
" ---------------------------------------------------------------------------
" The distinctive strict-equality operator
syn match hilowOperator "?="
" Comparison / not-comparison family
syn match hilowOperator "!=\|!<\|!>\|<=\|>=\|<<\|>>\|=>\|\.\."
syn match hilowOperator "[+\-*/%]=\?"
syn match hilowOperator "[<>=&|^~?@]"

" ---------------------------------------------------------------------------
" Function definitions / calls (light touch)
" ---------------------------------------------------------------------------
" Highlight the name following 'function'
syn match hilowFunction "\<function\s\+\zs\w\+"
" Highlight a name immediately followed by '(' as a call
syn match hilowFuncCall "\<\w\+\ze\s*("

" ---------------------------------------------------------------------------
" Highlight links
" ---------------------------------------------------------------------------
hi def link hilowComment      Comment
hi def link hilowTodo         Todo
hi def link hilowMode         Structure
hi def link hilowKeyword      Keyword
hi def link hilowConditional  Conditional
hi def link hilowRepeat       Repeat
hi def link hilowStorage      StorageClass
hi def link hilowVerify       PreProc
hi def link hilowWatcher      Statement
hi def link hilowModifier     Special
hi def link hilowType         Type
hi def link hilowBoolean      Boolean
hi def link hilowCurrency     Constant
hi def link hilowNumber       Number
hi def link hilowFloat        Float
hi def link hilowMoney        Number
hi def link hilowDuration     Number
hi def link hilowString       String
hi def link hilowFString      String
hi def link hilowFStringPrefix Special
hi def link hilowEscape       SpecialChar
hi def link hilowInterp       Normal
hi def link hilowInterpDelim  Special
hi def link hilowOperator     Operator
hi def link hilowFunction     Function
hi def link hilowFuncCall     Function

let b:current_syntax = "hilow"

