# Ornn
The blacksmith crafts material into verifiers.

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
   - assignment
     `// domains[0] = point^trace_length - 1` 
4. generate new code

5. write to file


## Install

## How To Use

## Configuration
