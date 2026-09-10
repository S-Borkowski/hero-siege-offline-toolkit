#pragma once
#include <string_view>
#include <string>
#include "yytk_helpers.hpp"

namespace HeroSiege::Hooks {

#ifdef HS_SDK_HAS_YYTK

using ::YYTK::YYTKInterface;
using ::YYTK::CScript;
using ::YYTK::PFUNC_YYGMLScript;
using ::YYTK::RValue;
using ::YYTK::CInstance;

/**
 * Installs a function pointer hook on a named GML GameMaker script routine.
 * Saves the original script function trampoline in outOriginalFunc.
 */
inline bool InstallScriptHook(
    YYTKInterface* yytk,
    std::string_view scriptName,
    PFUNC_YYGMLScript hookFunction,
    PFUNC_YYGMLScript* outOriginalFunc = nullptr
) {
    if (!yytk || !hookFunction) return false;
    
    PVOID routinePtr = nullptr;
    std::string nameStr(scriptName);
    if (!AurieSuccess(yytk->GetNamedRoutinePointer(nameStr.c_str(), &routinePtr)) || !routinePtr) {
        return false;
    }

    auto* script = reinterpret_cast<CScript*>(routinePtr);
    if (!script || !script->m_Functions) return false;

    if (outOriginalFunc) {
        *outOriginalFunc = script->m_Functions->m_ScriptFunction;
    }

    script->m_Functions->m_ScriptFunction = hookFunction;
    return true;
}

#define HS_INSTALL_SCRIPT_HOOK(yytk, scriptName, hookFunc, origFunc) \
    ::HeroSiege::Hooks::InstallScriptHook(yytk, scriptName, hookFunc, &origFunc)

#endif

} // namespace HeroSiege::Hooks
