import ScydeCore
import SwiftUI
import UIKit
import WebKit

/// What grows out of the island (DESIGN.md §6.3 Swell).
enum Panel: Equatable {
    case none, menu, shield, omnibox
}

/// The island at the bottom (DESIGN.md §8): mark (menu) · back · address · Shield pebble · tabs.
/// Swipe left/right switches tabs, swipe up opens the tab grid; scrolling down shrinks it to the pebble.
struct IslandView: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    @Binding var panel: Panel
    @Binding var showTabs: Bool
    @Binding var text: String
    @FocusState private var focused: Bool
    @State private var drag: CGFloat = 0

    var body: some View {
        let tab = browser.active
        if (tab?.pebble ?? false) && panel == .none {
            pebble(tab)
                .transition(.scale(scale: ScydeMotion.bloomScale).combined(with: .opacity))
        } else {
            island(tab)
                .transition(.scale(scale: ScydeMotion.bloomScale).combined(with: .opacity))
        }
    }

    // MARK: full island

    private func island(_ tab: Tab?) -> some View {
        let c = palette.colors
        let editing = panel == .omnibox
        return HStack(spacing: ScydeSpace.s2) {
            if editing {
                TextField(String(localized: "omnibox.placeholder"), text: $text)
                    .focused($focused)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.webSearch)
                    .submitLabel(.go)
                    .font(.body)
                    .padding(.horizontal, ScydeSpace.s12)
                    .frame(height: ScydeSpace.s40)
                    .background(c.islandRaised, in: Capsule())
                    .onSubmit {
                        browser.go(text)
                        close()
                    }
                    .onAppear { focused = true }
                Button(String(localized: "omnibox.cancel")) { close() }
                    .font(.subheadline)
                    .padding(.horizontal, ScydeSpace.s8)
                    .frame(minHeight: ScydeSize.hitIos)
            } else {
                Button { toggle(.menu) } label: {
                    Group {
                        if tab?.isPrivate == true {
                            Image(systemName: "eye.slash").font(.body.weight(.medium))
                        } else {
                            BrandMarkShape().fill(ScydeBrand.mark).frame(width: ScydeSpace.s24, height: ScydeSpace.s24)
                        }
                    }
                    .frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                }
                .accessibilityLabel(String(localized: "island.menu"))

                if tab?.canGoBack == true {
                    Button { tab?.webView.goBack() } label: {
                        Image(systemName: "chevron.left").font(.body.weight(.medium)).frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                    }
                    .accessibilityLabel(String(localized: "nav.back"))
                    .transition(.scale(scale: ScydeMotion.bloomScale).combined(with: .opacity))
                }

                address(tab).frame(maxWidth: .infinity)

                Button { toggle(.shield) } label: {
                    HStack(spacing: ScydeSpace.s6) {
                        Circle()
                            .fill(browser.shieldOn(for: tab) && browser.shieldReady ? c.islandShield : c.islandTextSecondary)
                            .frame(width: ScydeSpace.s8, height: ScydeSpace.s8)
                    }
                    .padding(.horizontal, ScydeSpace.s12)
                    .frame(height: ScydeSpace.s40)
                    .background(panel == .shield ? c.islandRaised : .clear, in: Capsule())
                    .frame(minWidth: ScydeSize.hitIos, minHeight: ScydeSize.hitIos)
                }
                .accessibilityLabel(String(localized: "shield.label"))

                Button { showTabs = true } label: {
                    Text("\(browser.tabs.filter { $0.isPrivate == (tab?.isPrivate ?? false) }.count)")
                        .font(.system(.footnote, design: .monospaced).weight(.medium))
                        .frame(minWidth: ScydeSpace.s24 + ScydeSpace.s2, minHeight: ScydeSpace.s24 + ScydeSpace.s2)
                        .overlay(RoundedRectangle(cornerRadius: ScydeRadius.xs).stroke(c.islandText, lineWidth: 1.5))
                        .frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                }
                .accessibilityLabel(String(localized: "mobile.tabs"))
                .contextMenu {
                    Button { browser.newTab(isPrivate: false) } label: { Label(String(localized: "island.new_tab"), systemImage: "plus") }
                    Button { browser.newTab(isPrivate: true) } label: { Label(String(localized: "menu.new_private_tab"), systemImage: "eye.slash") }
                    if let tab {
                        Button(role: .destructive) { browser.close(tab) } label: { Label(String(localized: "tab.close"), systemImage: "xmark") }
                    }
                }
            }
        }
        .foregroundStyle(c.islandText)
        .padding(.horizontal, ScydeSpace.s6)
        .frame(height: ScydeSize.islandMobileHeight)
        .background(islandFill(tab), in: RoundedRectangle(cornerRadius: ScydeRadius.island, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: ScydeRadius.island, style: .continuous).stroke(c.islandBorder, lineWidth: 1))
        .shadow(color: .black.opacity(0.35), radius: ScydeSpace.s20, y: ScydeSpace.s12)
        .overlay(alignment: .bottom) {
            if let tab, tab.loading, !editing {
                GeometryReader { g in
                    Capsule().fill(palette.accent.fill).frame(width: g.size.width * tab.progress, height: ScydeSpace.s2)
                }
                .frame(height: ScydeSpace.s2)
                .padding(.horizontal, ScydeSpace.s24)
            }
        }
        .offset(x: drag)
        // Simultaneous, so a drag that starts on a button (most of the island) still switches tabs.
        .simultaneousGesture(
            DragGesture(minimumDistance: ScydeSpace.s16)
                .onChanged { g in
                    guard !editing, abs(g.translation.width) > abs(g.translation.height) else { return }
                    drag = g.translation.width * 0.35 // follows the finger with resistance (transform only)
                }
                .onEnded { g in
                    withAnimation(ScydeMotion.springSmooth.animation) { drag = 0 }
                    guard !editing else { return }
                    if g.translation.height < -ScydeSpace.s40, abs(g.translation.height) > abs(g.translation.width) {
                        showTabs = true
                    } else if abs(g.translation.width) > ScydeSpace.s48 {
                        browser.switchTab(by: g.translation.width < 0 ? 1 : -1)
                        UIImpactFeedbackGenerator(style: .light).impactOccurred()
                    }
                }
        )
        .animation(ScydeMotion.springSmooth.animation, value: tab?.canGoBack)
        .animation(ScydeMotion.springSmooth.animation, value: editing)
    }

    @ViewBuilder private func address(_ tab: Tab?) -> some View {
        let c = palette.colors
        Button { startEditing(tab) } label: {
            HStack(spacing: ScydeSpace.s4) {
                if tab?.isPrivate == true {
                    Text(String(localized: "private.label"))
                        .font(.caption2.monospaced())
                        .padding(.horizontal, ScydeSpace.s6)
                        .frame(height: ScydeSpace.s20)
                        .background(c.islandRaised, in: Capsule())
                }
                if let d = tab?.display {
                    if d.scheme == "https" { Image(systemName: "lock.fill").font(.caption2).foregroundStyle(c.islandTextSecondary) }
                    Text(d.domain).lineLimit(1).truncationMode(.head)
                } else if let url = tab?.url, InternalPages.isInternal(url), url.host != "newtab" {
                    Text("scyde://\(url.host ?? "")").lineLimit(1)
                } else {
                    Text(String(localized: "omnibox.placeholder")).foregroundStyle(c.islandTextSecondary).lineLimit(1)
                }
            }
            .font(.body)
            .frame(maxWidth: .infinity, minHeight: ScydeSize.hitIos)
            .contentShape(Rectangle())
        }
        .accessibilityLabel(tab?.url?.absoluteString ?? String(localized: "omnibox.placeholder"))
        .contextMenu {
            if tab?.isWeb == true {
                Button { browser.copyLink() } label: { Label(String(localized: "menu.copy_link"), systemImage: "doc.on.doc") }
            }
            // Only `hasStrings` here: reading the string itself would ask for paste permission.
            if UIPasteboard.general.hasStrings {
                Button {
                    if let pasted = UIPasteboard.general.string, !pasted.isEmpty { browser.go(pasted) }
                } label: { Label(String(localized: "omnibox.paste_go"), systemImage: "doc.on.clipboard") }
            }
        }
    }

    // MARK: pebble (quiet while reading)

    private func pebble(_ tab: Tab?) -> some View {
        let c = palette.colors
        return Button {
            withAnimation(ScydeMotion.springSmooth.animation) { tab?.pebble = false }
        } label: {
            HStack(spacing: ScydeSpace.s8) {
                Circle().fill(browser.shieldOn(for: tab) ? c.islandShield : c.islandTextSecondary).frame(width: ScydeSpace.s8, height: ScydeSpace.s8)
                Text(tab?.display?.domain ?? "").font(.footnote).lineLimit(1)
            }
            .foregroundStyle(c.islandText)
            .padding(.horizontal, ScydeSpace.s16)
            .frame(height: ScydeSpace.s40)
            .background(islandFill(tab), in: Capsule())
            .overlay(Capsule().stroke(c.islandBorder, lineWidth: 1))
            .shadow(color: .black.opacity(0.3), radius: ScydeSpace.s12, y: ScydeSpace.s8)
        }
        .frame(maxWidth: .infinity)
        .accessibilityLabel(String(localized: "island.label"))
    }

    /// Aura tint: a little of the page's theme-color, only when the core says text stays AA.
    private func islandFill(_ tab: Tab?) -> Color {
        guard let hex = tab?.themeColor?.hex, let tinted = themeIslandTint(pageHex: hex, dark: palette.dark),
              let color = Color(hex: tinted) else { return palette.colors.islandBg }
        return color
    }

    private func toggle(_ p: Panel) {
        withAnimation(ScydeMotion.springSmooth.animation) { panel = panel == p ? .none : p }
    }

    private func startEditing(_ tab: Tab?) {
        text = tab?.editableURL ?? ""
        withAnimation(ScydeMotion.springSmooth.animation) { panel = .omnibox }
    }

    private func close() {
        focused = false
        withAnimation(ScydeMotion.springSmooth.animation) { panel = .none }
    }
}

// MARK: - panels

/// The panel above the island: control center, Shield or the omnibox suggestions.
struct PanelView: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    @Binding var panel: Panel
    @Binding var text: String

    var body: some View {
        let c = palette.colors
        Group {
            switch panel {
            case .menu: ControlCenter(close: close)
            case .shield: ShieldPanel(close: close)
            case .omnibox: OmniboxPanel(text: text, close: close)
            case .none: EmptyView()
            }
        }
        .foregroundStyle(c.textPrimary)
        .background(c.surfaceRaised, in: RoundedRectangle(cornerRadius: ScydeRadius.xl, style: .continuous))
        .shadow(color: .black.opacity(0.18), radius: ScydeSpace.s24, y: ScydeSpace.s12)
    }

    private func close() {
        withAnimation(ScydeMotion.springSmooth.animation) { panel = .none }
    }
}

/// Every page and browser action in one place (the "menu" of the island).
struct ControlCenter: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    let close: () -> Void

    var body: some View {
        let c = palette.colors
        let tab = browser.active
        VStack(spacing: ScydeSpace.s12) {
            HStack(spacing: ScydeSpace.s8) {
                navButton("chevron.left", "nav.back", enabled: tab?.canGoBack == true) { tab?.webView.goBack() }
                navButton("chevron.right", "nav.forward", enabled: tab?.canGoForward == true) { tab?.webView.goForward() }
                navButton(tab?.loading == true ? "xmark" : "arrow.clockwise", tab?.loading == true ? "nav.stop" : "nav.reload", enabled: tab != nil) {
                    browser.reloadOrStop()
                }
                if let url = tab?.url, tab?.isWeb == true {
                    ShareLink(item: url) {
                        Image(systemName: "square.and.arrow.up").frame(maxWidth: .infinity, minHeight: ScydeSize.hitIos)
                            .background(c.surfaceSunken, in: RoundedRectangle(cornerRadius: ScydeRadius.lg, style: .continuous))
                    }
                    .accessibilityLabel(String(localized: "mobile.share"))
                } else {
                    navButton("square.and.arrow.up", "mobile.share", enabled: false) {}
                }
            }
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: ScydeSpace.s8), count: 3), spacing: ScydeSpace.s8) {
                tile("plus", "island.new_tab") { browser.newTab(isPrivate: false) }
                tile("eye.slash", "menu.new_private_tab") { browser.newTab(isPrivate: true) }
                tile(tab?.bookmarked == true ? "star.fill" : "star", tab?.bookmarked == true ? "menu.bookmark.remove" : "menu.bookmark.add",
                     on: tab?.bookmarked == true, enabled: tab?.isWeb == true) { browser.toggleBookmark() }
                tile("text.magnifyingglass", "menu.find", enabled: tab?.isWeb == true) { browser.find() }
                tile("desktopcomputer", "menu.desktop_site", on: tab?.desktop == true, enabled: tab?.isWeb == true) { browser.toggleDesktop() }
                tile("doc.on.doc", "menu.copy_link", enabled: tab?.isWeb == true) { browser.copyLink() }
                tile("clock", "library.tab.history") { browser.open(page: "library#history") }
                tile("book", "library.tab.bookmarks") { browser.open(page: "library#bookmarks") }
                tile("slider.horizontal.3", "rail.settings") { browser.open(page: "settings") }
            }
        }
        .padding(ScydeSpace.s12)
    }

    private func navButton(_ icon: String, _ key: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        let c = palette.colors
        return Button {
            action()
            close()
        } label: {
            Image(systemName: icon).font(.body.weight(.medium))
                .frame(maxWidth: .infinity, minHeight: ScydeSize.hitIos)
                .background(c.surfaceSunken, in: RoundedRectangle(cornerRadius: ScydeRadius.lg, style: .continuous))
                .foregroundStyle(enabled ? c.textPrimary : c.textTertiary)
        }
        .disabled(!enabled)
        .accessibilityLabel(String(localized: String.LocalizationValue(key)))
    }

    private func tile(_ icon: String, _ key: String, on: Bool = false, enabled: Bool = true, action: @escaping () -> Void) -> some View {
        let c = palette.colors
        return Button {
            action()
            close()
        } label: {
            VStack(alignment: .leading, spacing: ScydeSpace.s8) {
                Image(systemName: icon).font(.title3).foregroundStyle(on ? palette.accent.onFill : c.textSecondary)
                Text(String(localized: String.LocalizationValue(key))).font(.footnote).lineLimit(2).multilineTextAlignment(.leading)
            }
            .frame(maxWidth: .infinity, minHeight: ScydeSpace.s80, alignment: .leading)
            .padding(ScydeSpace.s12)
            .foregroundStyle(on ? palette.accent.onFill : (enabled ? c.textPrimary : c.textTertiary))
            .background(on ? palette.accent.fill : c.surfaceSunken, in: RoundedRectangle(cornerRadius: ScydeRadius.lg, style: .continuous))
        }
        .disabled(!enabled)
    }
}

/// Shield for the current site: one switch, what it means, and the way to the details.
struct ShieldPanel: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    let close: () -> Void

    var body: some View {
        let c = palette.colors
        let tab = browser.active
        let on = browser.shieldOn(for: tab)
        VStack(alignment: .leading, spacing: ScydeSpace.s16) {
            Capsule().fill(c.borderStrong).frame(width: ScydeSpace.s40, height: ScydeSpace.s4).frame(maxWidth: .infinity)
            if let site = tab?.site {
                Toggle(isOn: Binding(get: { on }, set: { browser.setShield($0, site: site) })) {
                    VStack(alignment: .leading, spacing: ScydeSpace.s4) {
                        Text(String(format: String(localized: "shield.panel.title"), site)).font(.title3.weight(.medium))
                        Text(String(localized: on ? "shield.panel.on" : "shield.panel.off")).font(.footnote.monospaced())
                            .foregroundStyle(c.textSecondary)
                    }
                }
                .tint(c.statusShield)
                .accessibilityLabel(String(localized: "shield.panel.toggle"))
            } else {
                Text(String(localized: "shield.panel.on")).font(.title3.weight(.medium))
            }
            Divider()
            Text(String(localized: "shield.panel.webkit")).font(.footnote).foregroundStyle(c.textSecondary)
            Button {
                browser.open(page: "shields")
                close()
            } label: {
                HStack {
                    Text(String(localized: "shield.panel.open"))
                    Spacer()
                    Image(systemName: "chevron.right").foregroundStyle(c.textTertiary)
                }
                .frame(minHeight: ScydeSize.hitIos)
            }
        }
        .padding(ScydeSpace.s20)
    }
}

/// The omnibox panel (DESIGN.md §8 Suggestions) that is also the command bar: open tabs,
/// history and bookmarks, the search row and browser actions, all from one field.
struct OmniboxPanel: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    let text: String
    let close: () -> Void

    private struct Action: Identifiable {
        let id: String
        let icon: String
        let run: () -> Void
    }

    var body: some View {
        let q = text.trimmingCharacters(in: .whitespaces)
        let tabs = matchingTabs(q)
        let suggestions = q.isEmpty
            ? ((try? browser.places?.history(query: nil, limit: 5, offset: 0)) ?? []).map { ($0.url, $0.title, false) }
            : ((try? browser.places?.suggest(query: q, limit: 5)) ?? []).map { ($0.url, $0.title, $0.bookmarked) }
        let actions = matchingActions(q)
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                if !tabs.isEmpty {
                    header("omnibox.section.tabs")
                    ForEach(tabs) { tab in
                        row(icon: "square.on.square", title: tab.title, sub: tab.display?.domain ?? "", source: String(localized: "omnibox.switch_to_tab")) {
                            browser.activeID = tab.id
                        }
                    }
                }
                if !suggestions.isEmpty || !q.isEmpty {
                    header("omnibox.section.suggestions")
                    ForEach(Array(suggestions.enumerated()), id: \.offset) { _, s in
                        row(icon: s.2 ? "star" : "clock", title: s.1.isEmpty ? s.0 : s.1, sub: omniboxDisplayUrl(url: s.0)?.domain ?? s.0,
                            source: String(localized: s.2 ? "omnibox.suggest.bookmark" : "omnibox.suggest.history")) {
                            browser.go(s.0)
                        }
                    }
                    if !q.isEmpty {
                        row(icon: "magnifyingglass", title: String(format: String(localized: "omnibox.suggest.search"), q), sub: "", source: "") {
                            browser.go(q)
                        }
                    }
                }
                if !actions.isEmpty {
                    header("omnibox.section.actions")
                    FlowChips(actions: actions.map { ($0.id, $0.icon, { $0.run(); close() }) })
                        .padding(.horizontal, ScydeSpace.s16)
                        .padding(.bottom, ScydeSpace.s12)
                }
            }
            .padding(.vertical, ScydeSpace.s8)
        }
        .frame(maxHeight: UIScreen.main.bounds.height * 0.55)
        .fixedSize(horizontal: false, vertical: true)
    }

    private func matchingTabs(_ q: String) -> [Tab] {
        guard !q.isEmpty else { return [] }
        let needle = q.lowercased()
        return browser.tabs.filter {
            $0.id != browser.activeID && ($0.title.lowercased().contains(needle) || ($0.url?.absoluteString.lowercased().contains(needle) ?? false))
        }
        .prefix(3).map { $0 }
    }

    private func matchingActions(_ q: String) -> [Action] {
        let all: [Action] = [
            Action(id: "menu.new_private_tab", icon: "eye.slash") { browser.newTab(isPrivate: true) },
            Action(id: "island.new_tab", icon: "plus") { browser.newTab(isPrivate: false) },
            Action(id: "menu.find", icon: "text.magnifyingglass") { browser.find() },
            Action(id: "menu.desktop_site", icon: "desktopcomputer") { browser.toggleDesktop() },
            Action(id: "menu.bookmark.add", icon: "star") { browser.toggleBookmark() },
            Action(id: "library.tab.history", icon: "clock") { browser.open(page: "library#history") },
            Action(id: "library.tab.bookmarks", icon: "book") { browser.open(page: "library#bookmarks") },
            Action(id: "shield.label", icon: "shield") { browser.open(page: "shields") },
            Action(id: "rail.settings", icon: "slider.horizontal.3") { browser.open(page: "settings") },
        ]
        guard !q.isEmpty else { return Array(all.prefix(4)) }
        let needle = q.lowercased()
        return all.filter { String(localized: String.LocalizationValue($0.id)).lowercased().contains(needle) }
    }

    private func header(_ key: String) -> some View {
        Text(String(localized: String.LocalizationValue(key)))
            .font(.caption2.monospaced())
            .foregroundStyle(palette.colors.textTertiary)
            .padding(.horizontal, ScydeSpace.s16)
            .padding(.top, ScydeSpace.s8)
            .padding(.bottom, ScydeSpace.s4)
    }

    private func row(icon: String, title: String, sub: String, source: String, action: @escaping () -> Void) -> some View {
        let c = palette.colors
        return Button {
            action()
            close()
        } label: {
            HStack(spacing: ScydeSpace.s12) {
                Image(systemName: icon).foregroundStyle(c.textSecondary).frame(width: ScydeSpace.s24)
                VStack(alignment: .leading, spacing: ScydeSpace.s2) {
                    Text(title).lineLimit(1)
                    if !sub.isEmpty { Text(sub).font(.caption).foregroundStyle(palette.accent.ink).lineLimit(1) }
                }
                Spacer(minLength: ScydeSpace.s8)
                Text(source).font(.caption2.monospaced()).foregroundStyle(c.textTertiary).lineLimit(1)
            }
            .padding(.horizontal, ScydeSpace.s16)
            .frame(minHeight: ScydeSize.hitIos + ScydeSpace.s4)
            .contentShape(Rectangle())
        }
    }
}

/// Wrapping chips for omnibox actions.
struct FlowChips: View {
    @Environment(\.palette) private var palette
    let actions: [(String, String, () -> Void)]

    var body: some View {
        let c = palette.colors
        LazyVGrid(columns: [GridItem(.adaptive(minimum: ScydeSpace.s80 * 1.6), spacing: ScydeSpace.s8)], alignment: .leading, spacing: ScydeSpace.s8) {
            ForEach(actions, id: \.0) { a in
                Button(action: a.2) {
                    Label(String(localized: String.LocalizationValue(a.0)), systemImage: a.1)
                        .font(.footnote)
                        .lineLimit(1)
                        .padding(.horizontal, ScydeSpace.s12)
                        .frame(maxWidth: .infinity, minHeight: ScydeSize.hitIos, alignment: .leading)
                        .background(c.surfaceSunken, in: Capsule())
                }
            }
        }
    }
}

// MARK: - tab grid

/// All tabs as a grid (DESIGN.md §8: a full-screen shelf), with normal and private tabs apart.
struct TabGrid: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    @Binding var isPresented: Bool
    @State private var showPrivate = false

    var body: some View {
        let c = palette.colors
        let list = browser.tabs.filter { $0.isPrivate == showPrivate }
        NavigationStack {
            ScrollView {
                if list.isEmpty {
                    Text(String(localized: "private.hint")).font(.footnote).foregroundStyle(c.textSecondary)
                        .multilineTextAlignment(.center).padding(ScydeSpace.s40)
                }
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: ScydeSpace.s12)], spacing: ScydeSpace.s12) {
                    ForEach(list) { tab in
                        Button {
                            browser.activeID = tab.id
                            isPresented = false
                        } label: {
                            VStack(alignment: .leading, spacing: ScydeSpace.s8) {
                                RoundedRectangle(cornerRadius: ScydeRadius.sm).fill(c.surfaceSunken).aspectRatio(3 / 4, contentMode: .fit)
                                    .overlay(Text(tab.display?.domain ?? (tab.url?.host ?? "")).font(.caption.monospaced()).foregroundStyle(c.textTertiary))
                                Text(tab.title.isEmpty ? String(localized: "tab.new") : tab.title).font(.footnote).lineLimit(1)
                                    .foregroundStyle(c.textPrimary)
                            }
                            .padding(ScydeSpace.s8)
                            .background(c.surfaceRaised, in: RoundedRectangle(cornerRadius: ScydeRadius.md))
                            .overlay(
                                RoundedRectangle(cornerRadius: ScydeRadius.md)
                                    .stroke(tab.id == browser.activeID ? palette.accent.ring : .clear, lineWidth: 2)
                            )
                        }
                        .overlay(alignment: .topTrailing) {
                            Button { browser.close(tab) } label: {
                                Image(systemName: "xmark").font(.caption.weight(.semibold)).frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                            }
                            .foregroundStyle(c.textSecondary)
                            .accessibilityLabel(String(localized: "tab.close"))
                        }
                    }
                }
                .padding(ScydeSpace.s16)
            }
            .background(c.surfaceCanvas)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button(String(localized: "mobile.tabs.close_all"), role: .destructive) { browser.closeAll(isPrivate: showPrivate) }
                        .disabled(list.isEmpty)
                }
                ToolbarItem(placement: .principal) {
                    Picker(String(localized: "mobile.tabs"), selection: $showPrivate) {
                        Text(String(localized: "mobile.tabs")).tag(false)
                        Text(String(localized: "private.label")).tag(true)
                    }
                    .pickerStyle(.segmented)
                    .frame(maxWidth: ScydeSpace.s80 * 2.5)
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button(String(localized: "mobile.tabs.done")) { isPresented = false }.bold()
                }
                ToolbarItem(placement: .bottomBar) {
                    Button {
                        browser.newTab(isPrivate: showPrivate)
                        isPresented = false
                    } label: {
                        Label(String(localized: showPrivate ? "menu.new_private_tab" : "island.new_tab"), systemImage: "plus")
                    }
                }
            }
            .onAppear { showPrivate = browser.active?.isPrivate ?? false }
        }
        .presentationDetents([.large])
    }
}

// MARK: - pieces

/// A message that rises out of the island (DESIGN.md §8 Toast).
struct ToastView: View {
    @Environment(\.palette) private var palette
    let message: String

    var body: some View {
        let c = palette.colors
        Text(message)
            .font(.footnote)
            .foregroundStyle(c.islandText)
            .padding(.horizontal, ScydeSpace.s16)
            .frame(minHeight: ScydeSpace.s40)
            .background(c.islandBg, in: Capsule())
            .overlay(Capsule().stroke(c.islandBorder, lineWidth: 1))
            .accessibilityAddTraits(.updatesFrequently)
    }
}

/// The SCYDE mark (brand/icon/scyde.svg): two mirrored strokes, drawn from the same path.
struct BrandMarkShape: Shape {
    func path(in rect: CGRect) -> Path {
        var half = Path()
        half.move(to: CGPoint(x: 400, y: 82))
        half.addLine(to: CGPoint(x: 251, y: 82))
        half.addCurve(to: CGPoint(x: 68, y: 241), control1: CGPoint(x: 110, y: 82), control2: CGPoint(x: 68, y: 101))
        half.addCurve(to: CGPoint(x: 160, y: 382), control1: CGPoint(x: 68, y: 328), control2: CGPoint(x: 160, y: 382))
        half.addLine(to: CGPoint(x: 243, y: 308.5))
        half.addLine(to: CGPoint(x: 214, y: 242))
        half.closeSubpath()
        let center = CGPoint(x: 263.5, y: 284)
        let rotate = CGAffineTransform(translationX: center.x, y: center.y).rotated(by: .pi).translatedBy(x: -center.x, y: -center.y)
        var mark = half
        mark.addPath(half.applying(rotate))
        let box = mark.boundingRect
        let scale = min(rect.width / box.width, rect.height / box.height)
        let fit = CGAffineTransform(translationX: rect.midX, y: rect.midY)
            .scaledBy(x: scale, y: scale)
            .translatedBy(x: -box.midX, y: -box.midY)
        return mark.applying(fit)
    }
}

/// Peek: a live preview of a long-pressed link.
final class PeekController: UIViewController {
    private let url: URL
    private let configuration: WKWebViewConfiguration

    init(url: URL, configuration: WKWebViewConfiguration) {
        self.url = url
        self.configuration = configuration
        super.init(nibName: nil, bundle: nil)
        let size = UIScreen.main.bounds.size
        preferredContentSize = CGSize(width: size.width, height: size.height * 0.7)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    override func loadView() {
        let wv = WKWebView(frame: .zero, configuration: configuration)
        wv.load(URLRequest(url: url))
        view = wv
    }
}

extension UIColor {
    /// `#RRGGBB` in sRGB (extended values clamped).
    var hex: String? {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        guard getRed(&r, green: &g, blue: &b, alpha: &a) else { return nil }
        func ch(_ v: CGFloat) -> Int { Int((max(0, min(1, v)) * 255).rounded()) }
        return String(format: "#%02X%02X%02X", ch(r), ch(g), ch(b))
    }
}

extension Color {
    init?(hex: String) {
        let h = hex.hasPrefix("#") ? String(hex.dropFirst()) : hex
        guard h.count == 6, let v = UInt32(h, radix: 16) else { return nil }
        self.init(.sRGB, red: Double((v >> 16) & 0xFF) / 255, green: Double((v >> 8) & 0xFF) / 255, blue: Double(v & 0xFF) / 255, opacity: 1)
    }
}
