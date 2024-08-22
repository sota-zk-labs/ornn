# Ornn
The blacksmith who crafts materials into verifiers.

[![License](https://img.shields.io/github/license/sota-zk/ornn)](./LICENSE)

## Introduction

This repo contains tool for generating move codes

Strategy:  
1. read the sol file  
2. extract all comments  
3. parse comment
   1. trim the last `.` 
   - memory layout  
     `// [0x0, 0x20) - periodic_column/pedersen/points/x` -> `periodic_column_pedersen_points_x` : `0x0`
     - add to memory registry
   - assignment
   - `// Denominator for constraints: 'cpu/decode/opcode_range_check/bit', 'diluted_check/permutation/step0', 'diluted_check/step'`
     - extract the denominator for constraints
   - `// domains[0] = point^trace_length - 1`
     - parse the expression `point^trace_length - 1` to AST
     - generate code for the expression
     - add to memory registry
   - `// Constraint expression for cpu/update_registers/update_pc/tmp0: column8_row2 - cpu__decode__opcode_range_check__bit_9 * column3_row9.`
     - compute the constraint expression
     - * numerator
     - * denominator

4. generate new code

5. write to file


## Install

## How To Use

## Configuration

bug: current denominator is not correct
