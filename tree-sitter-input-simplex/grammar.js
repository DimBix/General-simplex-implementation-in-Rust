/**
 * @file InputSimplex grammar for tree-sitter
 * @author dimitri
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

export default grammar({
  name: "input_simplex",
  
  word: $ => $.identifier,

  rules: {
    source_file: $ => $.formula,

    formula: $ => prec.left(
      seq($.atom, 
      repeat(seq('and', $.atom)),
      optional(seq('and', 'parameters', $.colon , $.identifier, $.colon, $.inter, repeat(seq($.comma, $.identifier, $.colon, $.inter))))
      )
    ),

    atom: $ => seq(
      $.sum, 
      $.op, 
      $.sum
    ),

    sum: $ => seq(
      seq(optional('-'),$.term), 
      repeat(seq(choice('+','-'), $.term))
    ),

    op: $ => choice('=', '<=', '>='),

  term: $ => choice(
    seq(
      $.constant, 
      optional(seq($.identifier, repeat(seq($.mult, $.identifier)))),
    ),
    seq($.identifier, repeat(seq($.mult, $.identifier)))
  ),


    constant: $ => choice(
      $.float
    ),

    identifier: $ => /[a-z][a-z0-9]*/,
    

    fraction: $ => seq(
      $.inter, 
      $.div, 
      $.inter
    ),

    float: $ => seq(
      $.inter,
      $.point,
      $.inter
    ),

    inter: $ => /[0-9]+/,
    
    point : $ => '.',
    minus : $ => '-',
    div : $ => '/',
    colon : $ => ':', 
    comma : $ => ',',
    mult: $ => '*'
  }
});

