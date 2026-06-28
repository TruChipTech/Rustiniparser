// sample.cpp - using rustiniparser from C++.
//
// Build (after `cargo build --release` from the repo root):
//   g++ -std=c++11 -I../../include sample.cpp ../../target/release/librustiniparser.a -lpthread -ldl -lm -o sample_cpp
// or with the shared library:
//   g++ -std=c++11 -I../../include sample.cpp -L../../target/release -lrustiniparser -o sample_cpp
//   LD_LIBRARY_PATH=../../target/release ./sample_cpp

#include "rustiniparser.h"

#include <iostream>
#include <memory>
#include <stdexcept>
#include <string>

namespace ini {

// Thin RAII wrapper around the C handle so the document frees itself.
class Document {
public:
    Document() : doc_(ini_new()) {
        if (!doc_) throw std::bad_alloc();
    }

    static Document parse(const std::string &text) {
        IniDocument *d = ini_parse(text.c_str());
        if (!d) throw std::runtime_error("ini parse error");
        return Document(d);
    }

    std::string get_string(const std::string &section, const std::string &key,
                           const std::string &fallback = "") const {
        // First call sizes the buffer, second fills it.
        size_t n = ini_get_string(doc_.get(), section.c_str(), key.c_str(),
                                  fallback.c_str(), nullptr, 0);
        std::string out(n, '\0');
        ini_get_string(doc_.get(), section.c_str(), key.c_str(), fallback.c_str(),
                       &out[0], n + 1);
        return out;
    }

    long long get_int(const std::string &s, const std::string &k, long long fb = 0) const {
        return ini_get_int(doc_.get(), s.c_str(), k.c_str(), fb);
    }
    double get_double(const std::string &s, const std::string &k, double fb = 0.0) const {
        return ini_get_double(doc_.get(), s.c_str(), k.c_str(), fb);
    }
    bool get_bool(const std::string &s, const std::string &k, bool fb = false) const {
        return ini_get_bool(doc_.get(), s.c_str(), k.c_str(), fb) != 0;
    }
    bool has(const std::string &s, const std::string &k) const {
        return ini_has(doc_.get(), s.c_str(), k.c_str()) != 0;
    }
    size_t size() const { return ini_len(doc_.get()); }

    void set(const std::string &s, const std::string &k, const std::string &v) {
        ini_set(doc_.get(), s.c_str(), k.c_str(), v.c_str());
    }
    void set_int(const std::string &s, const std::string &k, long long v) {
        ini_set_int(doc_.get(), s.c_str(), k.c_str(), v);
    }

private:
    explicit Document(IniDocument *d) : doc_(d) {}
    struct Deleter { void operator()(IniDocument *d) const { ini_free(d); } };
    std::unique_ptr<IniDocument, Deleter> doc_;
};

} // namespace ini

int main() {
    const std::string config =
        "; application config\n"
        "app_name = Rustiniparser\n"
        "[network]\n"
        "host = \"example.com\"   ; inline comment\n"
        "port = 8080\n"
        "ratio = 0.25\n"
        "mask = 0xFF\n"
        "[server]\n"
        "enabled = yes\n";

    ini::Document doc = ini::Document::parse(config);

    std::cout << "app_name      = " << doc.get_string("", "app_name", "?") << "\n";
    std::cout << "network.host  = " << doc.get_string("network", "host", "localhost") << "\n";
    std::cout << "network.port  = " << doc.get_int("network", "port", -1) << "\n";
    std::cout << "network.mask  = " << doc.get_int("network", "mask", -1) << " (0xFF)\n";
    std::cout << "network.ratio = " << doc.get_double("network", "ratio") << "\n";
    std::cout << "server.enabled= " << std::boolalpha << doc.get_bool("server", "enabled") << "\n";
    std::cout << "entries       = " << doc.size() << "\n";

    doc.set_int("network", "port", 9090);
    doc.set("network", "gateway", "10.0.0.1");

    std::cout << "\nafter edits:\n";
    std::cout << "network.port    = " << doc.get_int("network", "port", -1) << "\n";
    std::cout << "network.gateway = " << doc.get_string("network", "gateway") << "\n";
    std::cout << "entries         = " << doc.size() << "\n";

    return 0; // doc frees itself
}
