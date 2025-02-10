#pragma once

#include <string.h>

#include "arm64/Parser.h"

Token** Optimize(Token** pTokens);

bool isLabel(Token* pToken);

bool isRegister(Token* pToken);

bool isImmediate(Token* pToken);

bool isDirective(Token* pToken);