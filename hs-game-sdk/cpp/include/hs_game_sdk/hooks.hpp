#pragma once
#include <string_view>
#include <string>
#include "yytk_helpers.hpp"

#ifdef HS_SDK_HAS_YYTK
// YYTK_Shared_Base.hpp includes <Aurie/shared.hpp>, so MmCreateHook and
// AurieModule are available wherever the YYToolkit headers are.
#include <windows.h>
#endif

namespace HeroSiege::Hooks {

#ifdef HS_SDK_HAS_YYTK

using ::YYTK::YYTKInterface;
using ::YYTK::CScript;
using ::YYTK::PFUNC_YYGMLScript;
using ::YYTK::RValue;
using ::YYTK::CInstance;

/**
 * How a script hook ended up installed.
 *
 * MEASURED 2026-09-12: a script-table swap alone catches only the calls the
 * game routes through the script table. Read-only inspection of the shipped
 * exe found direct native callers (compiled GML emitting `call rel32` straight
 * at the function) for drop and stat routines among others, so a table-only
 * install can report success while the game runs straight past the hook. The
 * same defect was REPORTED against ForgePact and fixed there; this shared
 * installer carries the correction so every consumer gets it.
 *
 * `Native` means both routes are covered. `TableOnly` means the detour could
 * not be installed and direct compiled-GML calls will bypass the hook - the
 * `note` says why, and callers should surface it rather than treat it as
 * success.
 */
enum class ScriptHookKind {
    Failed = 0,
    Native,
    TableOnly,
    /// A hook was already installed on this script; the recorded original was
    /// left untouched. Whether that first install got native interception is
    /// not something this call can know, so it does not claim it.
    AlreadyInstalled,
};

struct ScriptHookResult {
    ScriptHookKind kind = ScriptHookKind::Failed;
    /// Why the native detour was not used, or why installation failed.
    const char* note = "";

    [[nodiscard]] bool Installed() const { return kind != ScriptHookKind::Failed; }
    [[nodiscard]] bool IsNative() const { return kind == ScriptHookKind::Native; }
    explicit operator bool() const { return Installed(); }
};

/**
 * Installs an inline detour at `target` and hands back a trampoline that
 * reaches the untouched original.
 *
 * This is a seam: it defaults to Aurie's MmCreateHook, and tests substitute a
 * controlled implementation so the installer's decisions can be exercised
 * without an Aurie runtime or a live game.
 */
using NativeDetourFn = bool (*)(
    void* selfModule,
    const char* hookId,
    void* target,
    void* detour,
    void** outTrampoline
);

/**
 * Is `addr` committed, executable, and owned by `moduleBase`?
 *
 * Guards against patching an address that is not the game's to patch: if some
 * other hook already swapped this table entry, the entry points into that
 * module, and detouring it would hook our own code instead of the game's.
 */
[[nodiscard]] inline bool AddressIsExecutableInModule(void* moduleBase, const void* addr) {
    if (!moduleBase || !addr) return false;
    MEMORY_BASIC_INFORMATION mbi{};
    if (VirtualQuery(addr, &mbi, sizeof(mbi)) != sizeof(mbi)) return false;
    if (mbi.State != MEM_COMMIT) return false;
    if (mbi.AllocationBase != static_cast<PVOID>(moduleBase)) return false;
    const DWORD executable = PAGE_EXECUTE | PAGE_EXECUTE_READ
                           | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY;
    return (mbi.Protect & executable) != 0;
}

/// The default `NativeDetourFn`: Aurie's inline hook.
inline bool AurieNativeDetour(
    void* selfModule,
    const char* hookId,
    void* target,
    void* detour,
    void** outTrampoline
) {
    if (!selfModule || !hookId || !*hookId || !target || !detour || !outTrampoline) return false;
    PVOID trampoline = nullptr;
    const ::Aurie::AurieStatus status = ::Aurie::MmCreateHook(
        static_cast<::Aurie::AurieModule*>(selfModule),
        hookId,
        target,
        detour,
        &trampoline
    );
    if (!::Aurie::AurieSuccess(status) || !trampoline) return false;
    *outTrampoline = trampoline;
    return true;
}

struct ScriptHookOptions {
    /// The calling plugin's own AurieModule*. Required for native interception.
    void* selfModule = nullptr;
    /// Unique identifier for the detour. Required for native interception.
    const char* hookId = nullptr;
    /// Module the table entry must belong to. Defaults to the game executable.
    void* gameModuleBase = nullptr;
    /// Override the detour implementation. Defaults to AurieNativeDetour.
    NativeDetourFn detour = nullptr;
};

/**
 * Swaps a named GML script's table entry only, without installing a detour.
 *
 * DELIBERATELY LIMITED, and named so the limitation is visible at the call
 * site: calls the game makes directly into the function's own address bypass
 * this hook entirely. It exists for research - observing table-routed calls
 * without patching code - and is not what a shipped gameplay hook should use.
 * Prefer InstallScriptHook.
 *
 * `*outOriginalFunc` is written only on the first install, so installing twice
 * never overwrites the real original with the hook itself.
 */
inline ScriptHookResult InstallScriptHookTableOnly(
    YYTKInterface* yytk,
    std::string_view scriptName,
    PFUNC_YYGMLScript hookFunction,
    PFUNC_YYGMLScript* outOriginalFunc = nullptr
) {
    ScriptHookResult result{};
    if (!yytk || !hookFunction) {
        result.note = "null interface or hook function";
        return result;
    }

    PVOID routinePtr = nullptr;
    const std::string nameStr(scriptName);
    if (!::Aurie::AurieSuccess(yytk->GetNamedRoutinePointer(nameStr.c_str(), &routinePtr)) || !routinePtr) {
        result.note = "script not found";
        return result;
    }

    auto* script = reinterpret_cast<CScript*>(routinePtr);
    if (!script || !script->m_Functions) {
        result.note = "script has no function table";
        return result;
    }

    if (outOriginalFunc && !*outOriginalFunc) {
        *outOriginalFunc = script->m_Functions->m_ScriptFunction;
    }
    script->m_Functions->m_ScriptFunction = hookFunction;

    result.kind = ScriptHookKind::TableOnly;
    result.note = "table-only by request; direct compiled-GML calls bypass this hook";
    return result;
}

/**
 * Installs a hook on a named GML script routine, covering both call routes:
 * the script-table entry, and an inline detour at the function's own address
 * for the direct calls compiled GML makes.
 *
 * `*outOriginalFunc` becomes the trampoline, so a hook body that forwards
 * through it reaches the real original from either route and never re-enters
 * itself. It is required: without somewhere to keep the original, native
 * interception cannot be installed safely.
 *
 * Repeat installation is safe, which matters because a shared chokepoint can
 * be hooked from more than one call site:
 *
 *   1. The detour is attempted only on the first install (`!*outOriginalFunc`),
 *      the one moment the table is still guaranteed to hold the game's own
 *      function. A later install would read our own hook out of the table and
 *      patch that - an infinite loop.
 *   2. The table entry must be executable code inside the game module, or it
 *      is not ours to patch.
 *   3. A failed detour is not fatal: `*outOriginalFunc` falls back to the table
 *      entry, the hook stays table-only, and the result says so rather than
 *      pretending.
 */
inline ScriptHookResult InstallScriptHook(
    YYTKInterface* yytk,
    std::string_view scriptName,
    PFUNC_YYGMLScript hookFunction,
    PFUNC_YYGMLScript* outOriginalFunc,
    const ScriptHookOptions& options = {}
) {
    ScriptHookResult result{};
    if (!yytk || !hookFunction) {
        result.note = "null interface or hook function";
        return result;
    }
    if (!outOriginalFunc) {
        result.note = "an original-function pointer is required";
        return result;
    }

    PVOID routinePtr = nullptr;
    const std::string nameStr(scriptName);
    if (!::Aurie::AurieSuccess(yytk->GetNamedRoutinePointer(nameStr.c_str(), &routinePtr)) || !routinePtr) {
        result.note = "script not found";
        return result;
    }

    auto* script = reinterpret_cast<CScript*>(routinePtr);
    if (!script || !script->m_Functions) {
        result.note = "script has no function table";
        return result;
    }

    PFUNC_YYGMLScript tableEntry = script->m_Functions->m_ScriptFunction;
    const bool firstInstall = (*outOriginalFunc == nullptr);

    if (firstInstall) {
        result.kind = ScriptHookKind::TableOnly;

        NativeDetourFn detour = options.detour ? options.detour : &AurieNativeDetour;
        void* moduleBase = options.gameModuleBase
            ? options.gameModuleBase
            : static_cast<void*>(GetModuleHandleA(nullptr));

        if (!options.selfModule) {
            result.note = "no calling module given; native interception needs one";
        } else if (!options.hookId || !*options.hookId) {
            result.note = "no hook id given; native interception needs one";
        } else if (!AddressIsExecutableInModule(moduleBase, reinterpret_cast<const void*>(tableEntry))) {
            result.note = "table entry is not executable code inside the game module";
        } else {
            void* trampoline = nullptr;
            const bool ok = detour(
                options.selfModule,
                options.hookId,
                reinterpret_cast<void*>(tableEntry),
                reinterpret_cast<void*>(hookFunction),
                &trampoline
            );
            if (ok && trampoline) {
                *outOriginalFunc = reinterpret_cast<PFUNC_YYGMLScript>(trampoline);
                result.kind = ScriptHookKind::Native;
                result.note = "";
            } else {
                result.note = "native detour installation failed";
            }
        }

        if (result.kind != ScriptHookKind::Native) {
            // Still the game's own function: the table has not been swapped yet.
            *outOriginalFunc = tableEntry;
        }
    } else {
        // Already hooked. The table no longer holds the game's function, so
        // leave the recorded original alone.
        result.kind = ScriptHookKind::AlreadyInstalled;
        result.note = "already installed; original preserved";
    }

    script->m_Functions->m_ScriptFunction = hookFunction;
    return result;
}

/**
 * Convenience wrapper. `origFunc` must be a zero-initialised
 * PFUNC_YYGMLScript with static storage, so repeat installs stay safe.
 */
#define HS_INSTALL_SCRIPT_HOOK(yytk, scriptName, hookFunc, origFunc, options) \
    ::HeroSiege::Hooks::InstallScriptHook(yytk, scriptName, hookFunc, &origFunc, options)

#endif

} // namespace HeroSiege::Hooks
