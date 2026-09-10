#pragma once
#include <string_view>
#include <vector>
#include <string>
#include "objects.hpp"
#include "scripts.hpp"

#ifdef __has_include
#if __has_include(<YYToolkit/YYTK_Shared.hpp>)
#include <YYToolkit/YYTK_Shared.hpp>
#define HS_SDK_HAS_YYTK 1
#endif
#endif

namespace HeroSiege::YYTK {

#ifdef HS_SDK_HAS_YYTK
using ::YYTK::RValue;
using ::YYTK::CInstance;
using ::YYTK::YYTKInterface;
using ::YYTK::CScript;
using ::YYTK::PFUNC_YYGMLScript;

inline RValue CallGameScript(YYTKInterface* yytk, std::string_view scriptName, const std::vector<RValue>& args = {}) {
    if (!yytk) return RValue();
    return yytk->CallGameScript(std::string(scriptName), args);
}

inline RValue GetInstanceVariable(YYTKInterface* yytk, const RValue& instance, std::string_view varName) {
    if (!yytk) return RValue();
    return yytk->CallBuiltin("variable_instance_get", { instance, RValue(std::string(varName)) });
}

inline void SetInstanceVariable(YYTKInterface* yytk, const RValue& instance, std::string_view varName, const RValue& value) {
    if (!yytk) return;
    yytk->CallBuiltin("variable_instance_set", { instance, RValue(std::string(varName)), value });
}

inline bool InstanceHasVariable(YYTKInterface* yytk, const RValue& instance, std::string_view varName) {
    if (!yytk) return false;
    return yytk->CallBuiltin("variable_instance_exists", { instance, RValue(std::string(varName)) }).ToBoolean();
}

inline RValue GetStructVariable(YYTKInterface* yytk, const RValue& structVal, std::string_view varName) {
    if (!yytk || structVal.m_Kind != ::YYTK::VALUE_OBJECT) return RValue();
    return yytk->CallBuiltin("variable_struct_get", { structVal, RValue(std::string(varName)) });
}

inline RValue GetStructVariable(YYTKInterface* yytk, const RValue& structVal, const char* varName) {
    if (!yytk || structVal.m_Kind != ::YYTK::VALUE_OBJECT || !varName) return RValue();
    return yytk->CallBuiltin("variable_struct_get", { structVal, RValue(std::string(varName)) });
}

inline RValue GetStructElement(YYTKInterface* yytk, const RValue& structVal, const RValue& keyVal) {
    if (!yytk || structVal.m_Kind != ::YYTK::VALUE_OBJECT) return RValue();
    return yytk->CallBuiltin("variable_struct_get", { structVal, keyVal });
}

inline void SetStructVariable(YYTKInterface* yytk, const RValue& structVal, std::string_view varName, const RValue& value) {
    if (!yytk || structVal.m_Kind != ::YYTK::VALUE_OBJECT) return;
    yytk->CallBuiltin("variable_struct_set", { structVal, RValue(std::string(varName)), value });
}

inline bool StructHasVariable(YYTKInterface* yytk, const RValue& structVal, std::string_view varName) {
    if (!yytk || structVal.m_Kind != ::YYTK::VALUE_OBJECT) return false;
    return yytk->CallBuiltin("variable_struct_exists", { structVal, RValue(std::string(varName)) }).ToBoolean();
}

inline RValue GetGlobalVariable(YYTKInterface* yytk, std::string_view varName) {
    if (!yytk) return RValue();
    return yytk->CallBuiltin("variable_global_get", { RValue(std::string(varName)) });
}

inline void SetGlobalVariable(YYTKInterface* yytk, std::string_view varName, const RValue& value) {
    if (!yytk) return;
    yytk->CallBuiltin("variable_global_set", { RValue(std::string(varName)), value });
}

inline bool GlobalHasVariable(YYTKInterface* yytk, std::string_view varName) {
    if (!yytk) return false;
    return yytk->CallBuiltin("variable_global_exists", { RValue(std::string(varName)) }).ToBoolean();
}

inline int GetArrayLength(YYTKInterface* yytk, const RValue& arrayVal) {
    if (!yytk || arrayVal.m_Kind != ::YYTK::VALUE_ARRAY) return 0;
    return static_cast<int>(yytk->CallBuiltin("array_length", { arrayVal }).ToDouble());
}

inline RValue GetArrayElement(YYTKInterface* yytk, const RValue& arrayVal, int index) {
    if (!yytk || arrayVal.m_Kind != ::YYTK::VALUE_ARRAY) return RValue();
    return yytk->CallBuiltin("array_get", { arrayVal, RValue(static_cast<double>(index)) });
}

#endif

} // namespace HeroSiege::YYTK
