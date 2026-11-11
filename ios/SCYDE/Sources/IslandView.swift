import ScydeCore
import SwiftUI
import UIKit

/// The island at the bottom (DESIGN.md §8 "Phone: island"): back · address · Shield · tabs · menu.
/// Swipe left/right switches tabs; swipe up opens the tab grid.
struct IslandView: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    @Binding var showTabs: Bool
    @State private var editing = false
    @State private var text = ""
    @FocusState private var focused: Bool

    var body: some View {
        let c = palette.colors
        HStack(spacing: ScydeSpace.s4) {
            if !editing {
                Button { browser.active?.webView.goBack() } label: {
                    Image(systemName: "chevron.left").frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                }
                .disabled(!(browser.active?.canGoBack ?? false))
                .accessibilityLabel(String(localized: "nav.back"))
            }

            address
                .frame(maxWidth: .infinity)

            if !editing {
                Button { browser.open(page: "shields") } label: {
                    Circle().fill(browser.shieldReady ? c.islandShield : c.islandTextSecondary)
                        .frame(width: ScydeSpace.s8, height: ScydeSpace.s8)
                        .frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                }
                .accessibilityLabel(String(localized: "shield.label"))

                Button { showTabs = true } label: {
                    Text("\(browser.tabs.count)")
                        .font(.system(.footnote, design: .monospaced).weight(.medium))
                        .frame(minWidth: ScydeSpace.s24, minHeight: ScydeSpace.s24)
                        .overlay(RoundedRectangle(cornerRadius: ScydeRadius.xs).stroke(c.islandText, lineWidth: 1.5))
                        .frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                }
                .accessibilityLabel(String(localized: "mobile.tabs"))

                Menu {
                    Button(String(localized: "island.new_tab")) { browser.newTab() }
                    Button(String(localized: "library.tab.history")) { browser.open(page: "library") }
                    Button(String(localized: "shield.label")) { browser.open(page: "shields") }
                    Button(String(localized: "rail.settings")) { browser.open(page: "settings") }
                    if let url = browser.active?.url, !InternalPages.isInternal(url) {
                        ShareLink(String(localized: "mobile.share"), item: url)
                    }
                } label: {
                    Image(systemName: "ellipsis").frame(width: ScydeSize.hitIos, height: ScydeSize.hitIos)
                }
                .accessibilityLabel(String(localized: "mobile.menu"))
            }
        }
        .foregroundStyle(c.islandText)
        .padding(.horizontal, ScydeSpace.s8)
        .frame(height: ScydeSize.islandMobileHeight)
        .background(c.islandBg, in: RoundedRectangle(cornerRadius: ScydeRadius.island, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: ScydeRadius.island, style: .continuous).stroke(c.islandBorder, lineWidth: 1))
        .shadow(color: .black.opacity(0.35), radius: ScydeSpace.s20, y: ScydeSpace.s12)
        .overlay(alignment: .bottom) {
            if let p = browser.active?.progress, browser.active?.loading == true {
                GeometryReader { g in
                    Capsule().fill(palette.accent.fill).frame(width: g.size.width * p, height: ScydeSpace.s2)
                }
                .frame(height: ScydeSpace.s2)
                .padding(.horizontal, ScydeSpace.s24)
            }
        }
        .gesture(
            DragGesture(minimumDistance: ScydeSpace.s24).onEnded { g in
                guard !editing else { return }
                if g.translation.height < -ScydeSpace.s40 { showTabs = true }
                else if abs(g.translation.width) > ScydeSpace.s48 { switchTab(by: g.translation.width < 0 ? 1 : -1) }
            }
        )
        .animation(ScydeMotion.springSmooth.animation, value: editing)
    }

    @ViewBuilder private var address: some View {
        let c = palette.colors
        if editing {
            TextField(String(localized: "omnibox.placeholder"), text: $text)
                .focused($focused)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .keyboardType(.webSearch)
                .submitLabel(.go)
                .font(.body)
                .padding(.horizontal, ScydeSpace.s12)
                .frame(height: ScydeSize.hitIos)
                .background(c.islandRaised, in: Capsule())
                .onSubmit {
                    browser.go(text)
                    editing = false
                }
                .onChange(of: focused) { f in if !f { editing = false } }
                .onAppear { focused = true }
        } else {
            Button {
                text = browser.active?.editableURL ?? ""
                editing = true
            } label: {
                HStack(spacing: ScydeSpace.s4) {
                    if let d = browser.active?.display {
                        if d.scheme == "https" { Image(systemName: "lock.fill").font(.caption2).foregroundStyle(c.islandTextSecondary) }
                        Text(d.domain).lineLimit(1).truncationMode(.head)
                    } else if let url = browser.active?.url, InternalPages.isInternal(url), url.host != "newtab" {
                        Text("scyde://\(url.host ?? "")").lineLimit(1)
                    } else {
                        Text(String(localized: "omnibox.placeholder")).foregroundStyle(c.islandTextSecondary).lineLimit(1)
                    }
                }
                .font(.body)
                .frame(maxWidth: .infinity, minHeight: ScydeSize.hitIos)
                .contentShape(Rectangle())
            }
            .accessibilityLabel(browser.active?.url?.absoluteString ?? String(localized: "omnibox.placeholder"))
        }
    }

    private func switchTab(by delta: Int) {
        guard let i = browser.tabs.firstIndex(where: { $0.id == browser.activeID }), !browser.tabs.isEmpty else { return }
        let j = (i + delta + browser.tabs.count) % browser.tabs.count
        browser.activeID = browser.tabs[j].id
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
    }
}

/// All tabs as a grid (DESIGN.md §8: a full-screen shelf).
struct TabGrid: View {
    @EnvironmentObject var browser: Browser
    @Environment(\.palette) private var palette
    @Binding var isPresented: Bool

    var body: some View {
        let c = palette.colors
        NavigationStack {
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: ScydeSpace.s12)], spacing: ScydeSpace.s12) {
                    ForEach(browser.tabs) { tab in
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
            .navigationTitle(String(localized: "mobile.tabs"))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .topBarLeading) {
                    Button(String(localized: "mobile.tabs.close_all"), role: .destructive) { browser.closeAll() }
                }
                ToolbarItem(placement: .principal) {
                    Button { browser.newTab(); isPresented = false } label: { Image(systemName: "plus") }
                        .accessibilityLabel(String(localized: "island.new_tab"))
                }
                ToolbarItem(placement: .topBarTrailing) {
                    Button(String(localized: "mobile.tabs.done")) { isPresented = false }.bold()
                }
            }
        }
        .presentationDetents([.large])
    }
}
